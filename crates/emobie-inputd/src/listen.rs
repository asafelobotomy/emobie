//! Trigger matching from one or more keyboard event devices.
//!
//! Key → char mapping uses libxkbcommon (session layout via XKB_DEFAULT_*).
//! Expansion runs on key *release* of the completing key so the focused app
//! has committed the trigger before we erase it.

use evdev::{Device, InputEventKind, Key};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::inject;
use crate::keymap::KeymapState;
use crate::matcher::TriggerTrie;

struct PendingExpand {
    erase: usize,
    expansion: String,
    trigger: String,
    key_code: u16,
}

static PENDING_HOLDER: OnceLock<Arc<Mutex<Option<PendingExpand>>>> = OnceLock::new();

/// Drop any pending expand (e.g. when expansion is disabled).
pub fn clear_pending() {
    if let Some(pending) = PENDING_HOLDER.get() {
        if let Ok(mut guard) = pending.lock() {
            *guard = None;
        }
    }
}

/// Cached keyboard-path scan so Status spam does not open every event device.
static LISTEN_CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
const LISTEN_CACHE_TTL: Duration = Duration::from_secs(2);

fn is_virtual_uinput(device: &Device) -> bool {
    let name = device.name().unwrap_or("").to_ascii_lowercase();
    name.contains("uinput")
        || name.contains("enigo")
        || name.contains("virtual")
        || name.contains("emobie")
}

fn is_keyboard(device: &Device) -> bool {
    if is_virtual_uinput(device) {
        return false;
    }
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
    if let Ok(cache) = LISTEN_CACHE.lock() {
        if let Some((at, ok)) = *cache {
            if at.elapsed() < LISTEN_CACHE_TTL {
                return ok;
            }
        }
    }
    let ok = !list_keyboard_paths().is_empty();
    if let Ok(mut cache) = LISTEN_CACHE.lock() {
        *cache = Some((Instant::now(), ok));
    }
    ok
}

fn fire_expand(
    pending: PendingExpand,
    trigger_committed: bool,
    enabled: &AtomicBool,
    buffer: &Mutex<String>,
) {
    if !enabled.load(Ordering::Relaxed) {
        return;
    }
    // Queue onto the inject worker — never call enigo on the listen thread.
    if let Err(err) = inject::expand_trigger(
        pending.erase,
        &pending.expansion,
        &pending.trigger,
        trigger_committed,
    ) {
        eprintln!("expand failed: {err}");
        // Queue full / worker down — put the trigger back so buffer stays aligned
        // with what the focused app still shows on screen.
        if let Ok(mut guard) = buffer.lock() {
            guard.push_str(&pending.trigger);
        }
    }
}

fn handle_key(
    key: Key,
    value: i32,
    keymap: &KeymapState,
    enabled: &AtomicBool,
    buffer: &Mutex<String>,
    trie: &Mutex<TriggerTrie>,
    pending: &Mutex<Option<PendingExpand>>,
) {
    let pressed = value != 0;
    keymap.update_key(key.code(), pressed);

    // Synthetic keys from our own inject — keep keymap in sync, ignore buffer.
    if inject::should_suppress_keys() {
        return;
    }

    // Key release: fire expansion for the key that completed the trigger.
    if value == 0 {
        let to_fire = {
            let Ok(mut guard) = pending.lock() else {
                return;
            };
            if guard.as_ref().is_some_and(|p| p.key_code == key.code()) {
                guard.take()
            } else {
                None
            }
        };
        if let Some(p) = to_fire {
            // Key already released — trigger text is committed; skip pre-delay.
            fire_expand(p, true, enabled, buffer);
        }
        return;
    }

    // Only initial presses update the match buffer (ignore autorepeat).
    if value != 1 {
        return;
    }

    // A new key while an expand is pending: flush the pending expand first so
    // the next character cannot race ahead of the replacement.
    {
        let to_fire = {
            let Ok(mut guard) = pending.lock() else {
                return;
            };
            guard.take()
        };
        if let Some(p) = to_fire {
            // Completing key still down — allow a short settle before erase.
            fire_expand(p, false, enabled, buffer);
        }
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
        if guard.chars().count() > 128 {
            let trim: String = guard
                .chars()
                .rev()
                .take(96)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            *guard = trim;
        }
        let matched = {
            let Ok(trie_guard) = trie.lock() else {
                return;
            };
            trie_guard.match_suffix(&guard)
        };
        if let Some((len, expansion)) = matched {
            let chars: Vec<char> = guard.chars().collect();
            let start = chars.len().saturating_sub(len);
            let trigger: String = chars[start..].iter().collect();
            for _ in 0..len {
                guard.pop();
            }
            Some((len, expansion, trigger))
        } else {
            None
        }
    };

    if let Some((len, expansion, trigger)) = hit {
        if let Ok(mut guard) = pending.lock() {
            *guard = Some(PendingExpand {
                erase: len,
                expansion,
                trigger,
                key_code: key.code(),
            });
        }
    }
}

fn spawn_device_thread(
    path: PathBuf,
    enabled: Arc<AtomicBool>,
    trie: Arc<Mutex<TriggerTrie>>,
    buffer: Arc<Mutex<String>>,
    pending: Arc<Mutex<Option<PendingExpand>>>,
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
                            &pending,
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
        let pending = Arc::new(Mutex::new(None));
        let _ = PENDING_HOLDER.set(pending.clone());
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
                    pending.clone(),
                    stop.clone(),
                    alive.clone(),
                );
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}
