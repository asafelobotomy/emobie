mod inject;
mod listen;
mod matcher;
mod protocol;

use matcher::TriggerTrie;
use protocol::{Request, Response};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn runtime_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("emobie");
    }
    PathBuf::from("/tmp/emobie")
}

fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("EMOBIE_INPUTD_SOCKET") {
        return PathBuf::from(path);
    }
    let system = PathBuf::from("/run/emobie/emobie-inputd.sock");
    if system.parent().is_some_and(|p| p.exists()) {
        return system;
    }
    runtime_dir().join("emobie-inputd.sock")
}

fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
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
    let _ = ensure_parent(&path);
    let _ = fs::remove_file(&path);

    let enabled = Arc::new(AtomicBool::new(false));
    let trie = Arc::new(Mutex::new(TriggerTrie::default()));
    let stop = Arc::new(AtomicBool::new(false));
    // Paste uses enigo (compositor / uinput backends). Listen needs evdev.
    let can_inject = true;
    let _ = inject::can_open_uinput();
    let can_listen = listen::can_listen();

    listen::spawn_listener(enabled.clone(), trie.clone(), stop.clone());

    let listener = UnixListener::bind(&path).unwrap_or_else(|err| {
        eprintln!("failed to bind {}: {err}", path.display());
        std::process::exit(1);
    });
    println!("emobie-inputd listening on {}", path.display());

    let stop_flag = stop.clone();
    let _ = ctrlc::set_handler(move || {
        stop_flag.store(true, Ordering::Relaxed);
    });

    for stream in listener.incoming() {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match stream {
            Ok(stream) => {
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
