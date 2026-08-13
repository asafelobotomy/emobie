mod inject;
mod listen;
mod matcher;
mod protocol;

use matcher::TriggerTrie;
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::getuid;
use protocol::{Request, Response};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn runtime_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("emobie");
    }
    // Tests / broken sessions only — production binds under XDG_RUNTIME_DIR.
    PathBuf::from(format!("/tmp/emobie-{}", getuid()))
}

fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("EMOBIE_INPUTD_SOCKET") {
        return PathBuf::from(path);
    }
    runtime_dir().join("emobie-inputd.sock")
}

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

fn handle_client(
    stream: UnixStream,
    enabled: &AtomicBool,
    trie: &Mutex<TriggerTrie>,
    can_inject: bool,
    can_listen: bool,
) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut writer = stream;
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let trimmed = line.trim().to_string();
        line.clear();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Request>(&trimmed) {
            Ok(Request::Status) => Response::status(
                can_inject,
                can_listen,
                enabled.load(Ordering::Relaxed),
                "emobie-inputd running",
            ),
            Ok(Request::SetEnabled { enabled: value }) => {
                enabled.store(value, Ordering::Relaxed);
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
            Ok(Request::SyncMatches { matches }) => {
                let pairs: Vec<(String, String)> = matches
                    .into_iter()
                    .map(|m| (m.trigger, m.expansion))
                    .collect();
                if let Ok(mut guard) = trie.lock() {
                    guard.load(&pairs);
                }
                Response::status(
                    can_inject,
                    can_listen,
                    enabled.load(Ordering::Relaxed),
                    &format!("synced {} matches", pairs.len()),
                )
            }
            Ok(Request::InjectPaste) => match inject::inject_ctrl_v() {
                Ok(()) => Response::status(
                    can_inject,
                    can_listen,
                    enabled.load(Ordering::Relaxed),
                    "paste injected",
                ),
                Err(err) => Response::err(&err),
            },
            Err(err) => Response::err(&format!("bad request: {err}")),
        };
        if let Ok(payload) = serde_json::to_string(&response) {
            let _ = writeln!(writer, "{payload}");
        }
    }
}

fn main() {
    let path = socket_path();
    if let Some(parent) = path.parent() {
        if let Err(err) = ensure_runtime_dir(parent) {
            eprintln!("failed to create {}: {err}", parent.display());
            std::process::exit(1);
        }
    }
    let _ = fs::remove_file(&path);

    let enabled = Arc::new(AtomicBool::new(false));
    let trie = Arc::new(Mutex::new(TriggerTrie::default()));
    let stop = Arc::new(AtomicBool::new(false));
    let can_inject = inject::can_inject();
    let can_listen = listen::can_listen();

    listen::spawn_listener(enabled.clone(), trie.clone(), stop.clone());

    let listener = UnixListener::bind(&path).unwrap_or_else(|err| {
        eprintln!("failed to bind {}: {err}", path.display());
        std::process::exit(1);
    });
    if let Err(err) = chmod_path(&path, 0o600) {
        eprintln!("warning: could not chmod socket: {err}");
    }
    println!("emobie-inputd listening on {}", path.display());

    let stop_flag = stop.clone();
    let sock_cleanup = path.clone();
    let _ = ctrlc::set_handler(move || {
        stop_flag.store(true, Ordering::Relaxed);
        let _ = fs::remove_file(&sock_cleanup);
    });

    for stream in listener.incoming() {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match stream {
            Ok(stream) => {
                if !peer_uid_allowed(&stream) {
                    eprintln!("rejected non-owner peer on socket");
                    continue;
                }
                let enabled = enabled.clone();
                let trie = trie.clone();
                std::thread::spawn(move || {
                    handle_client(stream, &enabled, &trie, can_inject, can_listen);
                });
            }
            Err(err) => eprintln!("accept error: {err}"),
        }
    }

    let _ = fs::remove_file(&path);
}
