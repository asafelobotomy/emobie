mod inject;
mod keymap;
mod listen;
mod matcher;
mod protocol;
mod session_env;
mod socket_path;
mod prefs_bootstrap;
mod state;

use matcher::TriggerTrie;
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::sys::stat::{umask, Mode};
use nix::unistd::getuid;
use protocol::{MatchRule, Request, Response};
use socket_path::{acquire_instance_lock, instance_lock_dir, resolve_socket_path};
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_REQUEST_BYTES: u64 = 512 * 1024;
const MAX_CLIENT_THREADS: usize = 32;

fn ensure_runtime_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut perms = fs::metadata(dir)?.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(dir, perms)?;
    Ok(())
}

fn chmod_path(path: &Path, mode: u32) -> std::io::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)
}

fn peer_uid_allowed(stream: &UnixStream) -> bool {
    match getsockopt(stream, PeerCredentials) {
        Ok(cred) => cred.uid() == getuid().as_raw(),
        Err(_) => false,
    }
}

fn read_request_line(
    reader: &mut BufReader<UnixStream>,
    line: &mut String,
) -> std::io::Result<bool> {
    line.clear();
    // Bound the read so a huge unterminated payload cannot OOM the daemon.
    let mut buf = Vec::new();
    let n = {
        let mut limited = reader.by_ref().take(MAX_REQUEST_BYTES + 1);
        limited.read_until(b'\n', &mut buf)?
    };
    if n == 0 {
        return Ok(false);
    }
    if buf.len() > MAX_REQUEST_BYTES as usize || !buf.ends_with(b"\n") {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "request too large or missing newline",
        ));
    }
    *line = String::from_utf8_lossy(&buf).into_owned();
    Ok(true)
}

fn persist_locked(enabled: &AtomicBool, matches: &[MatchRule]) {
    state::save(enabled.load(Ordering::Relaxed), matches);
}

struct ClientSlot<'a>(&'a AtomicUsize);

impl Drop for ClientSlot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn write_response(writer: &mut UnixStream, response: &Response) {
    if let Ok(payload) = serde_json::to_string(response) {
        let _ = writeln!(writer, "{payload}");
    }
}

fn handle_client(
    stream: UnixStream,
    enabled: &AtomicBool,
    trie: &Mutex<TriggerTrie>,
    stored: &Mutex<Vec<MatchRule>>,
    _clients: ClientSlot<'_>,
) {
    let writer_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut reader = BufReader::new(stream);
    let mut writer = writer_stream;
    let mut line = String::new();
    loop {
        match read_request_line(&mut reader, &mut line) {
            Ok(true) => {}
            Ok(false) => break,
            Err(err) if err.kind() == ErrorKind::InvalidData => {
                let can_inject = inject::can_inject();
                let can_listen = listen::can_listen();
                write_response(
                    &mut writer,
                    &Response::err(
                        can_inject,
                        can_listen,
                        enabled.load(Ordering::Relaxed),
                        "request too large",
                    ),
                );
                break;
            }
            Err(_) => break,
        }
        let trimmed = line.trim().to_string();
        line.clear();
        if trimmed.is_empty() {
            continue;
        }
        let can_inject = inject::can_inject();
        let can_listen = listen::can_listen();
        let enabled_now = enabled.load(Ordering::Relaxed);
        let response = match serde_json::from_str::<Request>(&trimmed) {
            Ok(Request::Status) => Response::status(
                can_inject,
                can_listen,
                enabled_now,
                if can_listen {
                    "emobie-inputd running"
                } else {
                    "running, but keyboard access missing — run setup-input-access.sh \
(ACLs usually avoid logout; otherwise log out/in once)"
                },
            ),
            Ok(Request::SetEnabled { enabled: value }) => {
                enabled.store(value, Ordering::Relaxed);
                inject::set_expand_enabled(value);
                if !value {
                    listen::clear_pending();
                }
                // Hold `stored` while saving so we cannot overwrite a concurrent
                // SyncMatches with a stale matches snapshot.
                match stored.lock() {
                    Ok(guard) => persist_locked(enabled, &guard),
                    Err(poisoned) => persist_locked(enabled, &poisoned.into_inner()),
                }
                Response::status(
                    can_inject,
                    can_listen,
                    value,
                    if value {
                        "expansion enabled"
                    } else {
                        "expansion disabled"
                    },
                )
            }
            Ok(Request::SyncMatches { matches }) => match state::validate_matches(&matches) {
                Ok(()) => {
                    let pairs = state::pairs_from_matches(&matches);
                    let count = pairs.len();
                    // Update stored + trie + disk under one critical section.
                    let sync_result = (|| -> Result<(), String> {
                        let mut stored_guard = stored
                            .lock()
                            .map_err(|_| "internal lock poisoned — try again".to_string())?;
                        let mut trie_guard = trie
                            .lock()
                            .map_err(|_| "internal lock poisoned — try again".to_string())?;
                        trie_guard.load(&pairs);
                        *stored_guard = matches;
                        persist_locked(enabled, &stored_guard);
                        Ok(())
                    })();
                    match sync_result {
                        Ok(()) => {
                            Response::status(
                                can_inject,
                                can_listen,
                                enabled.load(Ordering::Relaxed),
                                &format!("synced {count} matches"),
                            )
                        },
                        Err(err) => Response::err(can_inject, can_listen, enabled_now, &err),
                    }
                }
                Err(err) => Response::err(can_inject, can_listen, enabled_now, &err),
            },
            Ok(Request::InjectPaste) => match inject::inject_ctrl_v() {
                Ok(()) => Response::status(
                    can_inject,
                    can_listen,
                    enabled.load(Ordering::Relaxed),
                    "paste injected",
                ),
                Err(err) => Response::err(can_inject, can_listen, enabled_now, &err),
            },
            Err(err) => Response::err(
                can_inject,
                can_listen,
                enabled_now,
                &format!("bad request: {err}"),
            ),
        };
        write_response(&mut writer, &response);
    }
}

fn main() {
    let path = resolve_socket_path();
    let lock_dir = instance_lock_dir();
    // Always create the per-uid lock dir (and socket parent if different).
    if let Err(err) = ensure_runtime_dir(&lock_dir) {
        eprintln!("failed to create {}: {err}", lock_dir.display());
        std::process::exit(1);
    }
    if let Some(parent) = path.parent() {
        if parent != lock_dir.as_path() {
            if let Err(err) = ensure_runtime_dir(parent) {
                eprintln!("failed to create {}: {err}", parent.display());
                std::process::exit(1);
            }
        }
    }

    // Global per-uid lock — covers XDG, /run/emobie, and /tmp fallback sockets.
    let _instance_lock = match acquire_instance_lock() {
        Ok(lock) => lock,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    let _ = fs::remove_file(&path);

    // Only bootstrap from preferences.json when no state file exists yet.
    // An on-disk empty matches list means the user (or SyncMatches) cleared them.
    let (mut persisted, from_disk) = state::load();
    if !from_disk && prefs_bootstrap::apply_if_empty(&mut persisted) {
        state::save(persisted.enabled, &persisted.matches);
    }
    let enabled = Arc::new(AtomicBool::new(persisted.enabled));
    inject::set_expand_enabled(persisted.enabled);
    let trie = Arc::new(Mutex::new(TriggerTrie::default()));
    let stored = Arc::new(Mutex::new(persisted.matches.clone()));
    if let Ok(mut guard) = trie.lock() {
        guard.load(&state::pairs_from_matches(&persisted.matches));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let clients = Arc::new(AtomicUsize::new(0));

    // Set WAYLAND_DISPLAY once before any worker threads (set_var is not thread-safe).
    session_env::ensure_session_env();

    listen::spawn_listener(enabled.clone(), trie.clone(), stop.clone());

    // Restrict socket mode at creation time (avoid a brief wider window).
    let prev_umask = umask(Mode::from_bits_truncate(0o177));
    let listener = UnixListener::bind(&path).unwrap_or_else(|err| {
        let _ = umask(prev_umask);
        eprintln!("failed to bind {}: {err}", path.display());
        std::process::exit(1);
    });
    let _ = umask(prev_umask);
    if let Err(err) = chmod_path(&path, 0o600) {
        eprintln!("warning: could not chmod socket: {err}");
    }
    if let Err(err) = listener.set_nonblocking(true) {
        eprintln!("warning: could not set nonblocking accept: {err}");
    }
    println!(
        "emobie-inputd listening on {} (enabled={}, matches={})",
        path.display(),
        persisted.enabled,
        persisted.matches.len()
    );

    // Boot-time user units often start before KWin/GNOME creates wayland-0.
    // Wait in the background so Status works immediately after bind.
    // Enigo uses wayland_display_for_enigo() (read-only detect), not process env.
    thread::spawn(|| {
        session_env::wait_for_compositor(Duration::from_secs(45));
    });

    let stop_flag = stop.clone();
    let sock_cleanup = path.clone();
    let _ = ctrlc::set_handler(move || {
        stop_flag.store(true, Ordering::Relaxed);
        let _ = fs::remove_file(&sock_cleanup);
    });

    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                if !peer_uid_allowed(&stream) {
                    eprintln!("rejected non-owner peer on socket");
                    continue;
                }
                let active = clients.fetch_add(1, Ordering::AcqRel);
                if active >= MAX_CLIENT_THREADS {
                    clients.fetch_sub(1, Ordering::AcqRel);
                    eprintln!("rejecting client — too many connections");
                    continue;
                }
                let enabled = enabled.clone();
                let trie = trie.clone();
                let stored = stored.clone();
                let clients = clients.clone();
                thread::spawn(move || {
                    let slot = ClientSlot(&clients);
                    handle_client(stream, &enabled, &trie, &stored, slot);
                });
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                eprintln!("accept error: {err}");
                thread::sleep(Duration::from_millis(50));
            }
        }
    }

    let _ = fs::remove_file(&path);
}

#[cfg(test)]
mod tests {
    use super::MAX_REQUEST_BYTES;
    use std::io::{BufRead, BufReader, Cursor, Read};

    fn read_bounded(data: &[u8]) -> (usize, Vec<u8>) {
        let mut reader = BufReader::new(Cursor::new(data));
        let mut buf = Vec::new();
        let n = {
            let mut limited = reader.by_ref().take(MAX_REQUEST_BYTES + 1);
            limited.read_until(b'\n', &mut buf).unwrap()
        };
        (n, buf)
    }

    #[test]
    fn bounded_read_rejects_oversize_line() {
        let huge = "x".repeat(MAX_REQUEST_BYTES as usize + 2) + "\n";
        let (n, buf) = read_bounded(huge.as_bytes());
        assert!(n > 0);
        assert!(buf.len() > MAX_REQUEST_BYTES as usize || !buf.ends_with(b"\n"));
    }

    #[test]
    fn bounded_read_accepts_normal_line() {
        let (n, buf) = read_bounded(b"{\"cmd\":\"status\"}\n");
        assert!(n > 0);
        assert!(buf.ends_with(b"\n"));
        assert!(buf.len() <= MAX_REQUEST_BYTES as usize);
    }
}
