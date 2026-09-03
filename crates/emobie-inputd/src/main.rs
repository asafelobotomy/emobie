mod inject;
mod keymap;
mod listen;
mod matcher;
mod protocol;
mod rpc;
mod session_env;
mod socket_path;
mod prefs_bootstrap;
mod state;
mod uinput_kbd;

use matcher::TriggerTrie;
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::sys::stat::{umask, Mode};
use nix::unistd::getuid;
use rpc::{configure_client_stream, handle_client, ClientSlot, MAX_CLIENT_THREADS};
use socket_path::{acquire_instance_lock, instance_lock_dir, resolve_socket_path};
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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

fn peer_uid_allowed(stream: &std::os::unix::net::UnixStream) -> bool {
    match getsockopt(stream, PeerCredentials) {
        Ok(cred) => cred.uid() == getuid().as_raw(),
        // Fail closed — missing credentials must not look like the owner.
        Err(_) => false,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        if arg == "--version" || arg == "-V" {
            println!("emobie-inputd {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        if arg == "--help" || arg == "-h" {
            println!(
                "emobie-inputd {} — host input helper for emobie Expand\n\
                 Usage: emobie-inputd [--version]\n\
                 Talks over a per-user Unix socket (XDG_RUNTIME_DIR/emobie/).",
                env!("CARGO_PKG_VERSION")
            );
            return;
        }
        eprintln!("unknown argument: {arg} (try --help)");
        std::process::exit(2);
    }

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
                    eprintln!("rejected non-owner peer on socket (uid mismatch)");
                    continue;
                }
                configure_client_stream(&stream);
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
