//! Keyboard device discovery for trigger matching.

use evdev::{Device, Key};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

pub(super) fn list_keyboard_paths() -> Vec<PathBuf> {
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
