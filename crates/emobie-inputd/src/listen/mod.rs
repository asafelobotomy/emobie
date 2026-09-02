//! Trigger matching from one or more keyboard event devices.
//!
//! Key → char mapping uses libxkbcommon (session layout via XKB_DEFAULT_*).
//! Expansion runs on key *release* of the completing key so the focused app
//! has committed the trigger before we erase it.

mod devices;
mod keys;

pub use devices::can_listen;

use evdev::{Device, InputEventKind};
use keys::{expire_stale_pending, handle_key, trim_buffer, PendingExpand};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::keymap::KeymapState;
use crate::matcher::TriggerTrie;

use devices::list_keyboard_paths;

/// Hotplug scan interval — avoid opening every event node more often than needed.
const HOTPLUG_INTERVAL: Duration = Duration::from_secs(5);

static PENDING_HOLDER: OnceLock<Arc<Mutex<Option<PendingExpand>>>> = OnceLock::new();
static BUFFER_HOLDER: OnceLock<Arc<Mutex<String>>> = OnceLock::new();

/// Drop any pending expand (e.g. when expansion is disabled).
pub fn clear_pending() {
    if let Some(pending) = PENDING_HOLDER.get() {
        if let Ok(mut guard) = pending.lock() {
            *guard = None;
        }
    }
}

/// Push text back into the match buffer (e.g. when a queued expand is cancelled).
pub fn restore_to_buffer(text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(buffer) = BUFFER_HOLDER.get() {
        if let Ok(mut guard) = buffer.lock() {
            guard.push_str(text);
            trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
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
        let mut last_reload = Instant::now();
        let result = (|| -> Result<(), ()> {
            let mut device = Device::open(&path).map_err(|_| ())?;
            loop {
                if stop.load(Ordering::Relaxed) {
                    return Ok(());
                }
                if last_reload.elapsed() >= Duration::from_secs(30) {
                    keymap.reload_from_session();
                    last_reload = Instant::now();
                }
                expire_stale_pending(&pending, &buffer);
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
        let _ = BUFFER_HOLDER.set(buffer.clone());
        let alive = Arc::new(Mutex::new(HashSet::<PathBuf>::new()));
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let paths = list_keyboard_paths();
            if paths.is_empty() {
                thread::sleep(HOTPLUG_INTERVAL);
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
            thread::sleep(HOTPLUG_INTERVAL);
        }
    });
}
