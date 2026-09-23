//! Keyboard device discovery for trigger matching.

use evdev::{Device, Key};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Cached keyboard-path scan so Status spam does not open every event device.
static LISTEN_CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
const LISTEN_CACHE_TTL: Duration = Duration::from_secs(2);

/// Only our own injector is skipped. Remappers (keyd, kanata, kmonad) grab the
/// physical keyboard and re-emit through a virtual one — that virtual device is
/// where the typed text actually comes from, so it must be read.
fn is_own_injector(device: &Device) -> bool {
    let id = device.input_id();
    let ours = id.vendor() == crate::uinput_kbd::VENDOR_ID && id.product() == crate::uinput_kbd::PRODUCT_ID;
    ours || device.name().unwrap_or("") == crate::uinput_kbd::DEVICE_NAME
}

fn is_keyboard(device: &Device) -> bool {
    if is_own_injector(device) {
        return false;
    }
    device.supported_keys().is_some_and(|keys| {
        keys.contains(Key::KEY_A) && keys.contains(Key::KEY_Z) && keys.contains(Key::KEY_ENTER)
    })
}

/// Mice, touchpads and touchscreens — read only so a click can reset the
/// typed-text buffer.
fn is_pointer(device: &Device) -> bool {
    if is_own_injector(device) {
        return false;
    }
    device
        .supported_keys()
        .is_some_and(|keys| keys.contains(Key::BTN_LEFT) || keys.contains(Key::BTN_TOUCH))
}

/// Keyboards plus pointer devices (see `is_pointer`).
pub(super) fn list_input_paths() -> Vec<PathBuf> {
    list_paths(|device| is_keyboard(device) || is_pointer(device))
}

pub(super) fn list_keyboard_paths() -> Vec<PathBuf> {
    list_paths(is_keyboard)
}

fn list_paths(wanted: impl Fn(&Device) -> bool) -> Vec<PathBuf> {
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
            if wanted(&device) {
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
