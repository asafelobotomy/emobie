//! Trigger matching from one or more keyboard event devices.
//!
//! Key → char mapping uses libxkbcommon (session layout via XKB_DEFAULT_*).

use evdev::{Device, InputEventKind, Key};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::inject;
use crate::keymap::KeymapState;
use crate::matcher::TriggerTrie;

fn is_keyboard(device: &Device) -> bool {
    device.supported_keys().is_some_and(|keys| {
        keys.contains(Key::KEY_A) && keys.contains(Key::KEY_Z) && keys.contains(Key::KEY_ENTER)
    })
}

fn list_keyboard_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Ok(entries) = fs::read_dir("/dev/input") else {
        return paths;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("event") {
            continue;
        }
        if let Ok(device) = Device::open(&path) {
            if is_keyboard(&device) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

pub fn can_listen() -> bool {
    !list_keyboard_paths().is_empty()
}

fn handle_key(
    key: Key,
    value: i32,
    keymap: &KeymapState,
    enabled: &AtomicBool,
    buffer: &Mutex<String>,
    trie: &Mutex<TriggerTrie>,
) {
    let pressed = value != 0;
    keymap.update_key(key.code(), pressed);

    if value != 1 {
        return;
    }
    if !enabled.load(Ordering::Relaxed) {
        if let Ok(mut guard) = buffer.lock() {
            guard.clear();
        }
        return;
    }
    if key == Key::KEY_BACKSPACE {
        if let Ok(mut guard) = buffer.lock() {
            guard.pop();
        }
        return;
    }
    if key == Key::KEY_ENTER || key == Key::KEY_TAB {
        if let Ok(mut guard) = buffer.lock() {
            guard.clear();
        }
        return;
    }

    let Some(text) = keymap.key_utf8(key.code()) else {
        return;
    };
    let Some(ch) = text.chars().next() else {
        return;
    };
    if ch.is_control() {
        return;
    }

    let hit = {
        let Ok(mut guard) = buffer.lock() else {
            return;
        };
        guard.push(ch);
        if guard.len() > 128 {
            let trim: String = guard.chars().skip(guard.len() - 96).collect();
            *guard = trim;
        }
        let matched = {
            let Ok(trie_guard) = trie.lock() else {
                return;
            };
            trie_guard.match_suffix(&guard)
        };
        if let Some((len, expansion)) = matched {
            for _ in 0..len {
                guard.pop();
            }
            Some((len, expansion))
        } else {
            None
        }
    };

    if let Some((len, expansion)) = hit {
        // Never expand on the listen thread: enigo/backends can panic or hang.
        thread::spawn(move || {
            if let Err(err) = inject::expand_trigger(len, &expansion) {
                eprintln!("expand failed: {err}");
            }
        });
    }
}

fn spawn_device_thread(
    path: PathBuf,
    enabled: Arc<AtomicBool>,
    trie: Arc<Mutex<TriggerTrie>>,
    buffer: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    alive: Arc<Mutex<HashSet<PathBuf>>>,
) {
    thread::spawn(move || {
        let keymap = KeymapState::new();
        let result = (|| -> Result<(), ()> {
            let mut device = Device::open(&path).map_err(|_| ())?;
            loop {
                if stop.load(Ordering::Relaxed) {
                    return Ok(());
                }
                let events = match device.fetch_events() {
                    Ok(events) => events,
                    Err(_) => {
                        thread::sleep(Duration::from_millis(50));
                        return Err(());
                    }
                };
                for event in events {
                    if let InputEventKind::Key(key) = event.kind() {
                        handle_key(
                            key,
                            event.value(),
                            &keymap,
                            &enabled,
                            &buffer,
                            &trie,
                        );
                    }
                }
            }
        })();
        let _ = result;
        if let Ok(mut guard) = alive.lock() {
            guard.remove(&path);
        }
    });
}

pub fn spawn_listener(
    enabled: Arc<AtomicBool>,
    trie: Arc<Mutex<TriggerTrie>>,
    stop: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let buffer = Arc::new(Mutex::new(String::new()));
        let alive = Arc::new(Mutex::new(HashSet::<PathBuf>::new()));
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let paths = list_keyboard_paths();
            if paths.is_empty() {
                thread::sleep(Duration::from_secs(2));
                continue;
            }
            for path in paths {
                let already = alive
                    .lock()
                    .map(|g| g.contains(&path))
                    .unwrap_or(true);
                if already {
                    continue;
                }
                if let Ok(mut guard) = alive.lock() {
                    guard.insert(path.clone());
                }
                spawn_device_thread(
                    path,
                    enabled.clone(),
                    trie.clone(),
                    buffer.clone(),
                    stop.clone(),
                    alive.clone(),
                );
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}
