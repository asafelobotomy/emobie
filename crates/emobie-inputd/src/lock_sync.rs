//! Keep the injector keyboard's Num Lock and Caps Lock in step with the user's keyboard on Hyprland.
//!
//! Hyprland keeps a separate xkb state for every keyboard, so `emobie-inject` has its own Num Lock
//! and Caps Lock. That leaks into what apps see: when our device types (a modifier changes), apps
//! get *its* lock state, and on every focus change Hyprland sends apps the locks of all keyboards
//! OR-ed together. A device created with Num Lock on (`input:numlock_by_default`) while the user has
//! it off makes the numpad type digits with the light off; one left with Num Lock on keeps forcing it
//! back on after the user turns it off. So before each injection we read every keyboard's locks from
//! Hyprland's IPC and tap Num Lock / Caps Lock on our device until they match the keyboard the user
//! last typed on (Hyprland's "main" keyboard). When ours is main, nothing has been typed since our
//! last sync, so it already matches. Other compositors keep one lock state for the seat: no-op.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use evdev::Key;
use serde::Deserialize;

use crate::uinput_kbd::{UInputKeyboard, DEVICE_NAME};

const IPC_TIMEOUT: Duration = Duration::from_millis(250);
/// After creation Hyprland may not list our device yet.
const LIST_RETRIES: usize = 5;
const LIST_RETRY_GAP: Duration = Duration::from_millis(20);

#[derive(Debug, Deserialize)]
struct Devices {
    #[serde(default)]
    keyboards: Vec<KeyboardInfo>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub(crate) struct KeyboardInfo {
    pub name: String,
    #[serde(rename = "capsLock", default)]
    pub caps_lock: bool,
    #[serde(rename = "numLock", default)]
    pub num_lock: bool,
    #[serde(default)]
    pub main: bool,
}

/// Which lock keys to tap on our device: (num_lock, caps_lock). `None` when ours isn't listed yet.
pub(crate) fn plan(keyboards: &[KeyboardInfo]) -> Option<(bool, bool)> {
    let ours = keyboards.iter().find(|k| is_ours(&k.name))?;
    if ours.main {
        return Some((false, false));
    }
    let Some(truth) = keyboards.iter().find(|k| k.main && !is_ours(&k.name)) else {
        return Some((false, false));
    };
    Some((ours.num_lock != truth.num_lock, ours.caps_lock != truth.caps_lock))
}

/// Hyprland names devices in lower case with spaces as dashes, and a second device with the same
/// name gets a numeric suffix (`emobie-inject-1`).
fn is_ours(name: &str) -> bool {
    name == DEVICE_NAME
        || name
            .strip_prefix(DEVICE_NAME)
            .and_then(|rest| rest.strip_prefix('-'))
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

pub(crate) fn parse(json: &str) -> Option<Vec<KeyboardInfo>> {
    serde_json::from_str::<Devices>(json).ok().map(|d| d.keyboards)
}

fn socket_path() -> Option<PathBuf> {
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join("hypr").join(sig).join(".socket.sock"))
}

pub(crate) fn on_hyprland() -> bool {
    socket_path().is_some_and(|p| p.exists())
}

fn query_keyboards() -> Option<Vec<KeyboardInfo>> {
    let mut stream = UnixStream::connect(socket_path()?).ok()?;
    stream.set_read_timeout(Some(IPC_TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(IPC_TIMEOUT)).ok()?;
    stream.write_all(b"j/devices").ok()?;
    let mut out = String::new();
    stream.read_to_string(&mut out).ok()?;
    parse(&out)
}

/// Best effort: any IPC failure leaves the device as it is.
pub(crate) fn sync(kbd: &mut UInputKeyboard) {
    if !on_hyprland() {
        return;
    }
    for attempt in 0..LIST_RETRIES {
        if attempt > 0 {
            std::thread::sleep(LIST_RETRY_GAP);
        }
        let Some(keyboards) = query_keyboards() else {
            return;
        };
        let Some((num, caps)) = plan(&keyboards) else {
            continue;
        };
        if num && kbd.click(Key::KEY_NUMLOCK).is_err() {
            return;
        }
        if caps && kbd.click(Key::KEY_CAPSLOCK).is_err() {
            return;
        }
        if num || caps {
            eprintln!("emobie-inputd: matched lock keys to the main keyboard (num={num} caps={caps})");
        }
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kb(name: &str, num: bool, caps: bool, main: bool) -> KeyboardInfo {
        KeyboardInfo { name: name.into(), num_lock: num, caps_lock: caps, main }
    }

    #[test]
    fn parses_hyprctl_devices() {
        let json = r#"{"mice":[],"keyboards":[
            {"address":"0x1","name":"power-button","capsLock":false,"numLock":true,"main":false},
            {"address":"0x2","name":"at-translated-set-2-keyboard","capsLock":true,"numLock":false,"main":true},
            {"address":"0x3","name":"emobie-inject","capsLock":false,"numLock":true,"main":false}
        ],"switches":[]}"#;
        let kbs = parse(json).unwrap();
        assert_eq!(kbs.len(), 3);
        assert_eq!(plan(&kbs), Some((true, true)));
    }

    #[test]
    fn nothing_to_do_when_in_step() {
        let kbs = [kb("at-translated-set-2-keyboard", true, false, true), kb("emobie-inject", true, false, false)];
        assert_eq!(plan(&kbs), Some((false, false)));
    }

    #[test]
    fn ours_main_means_already_synced() {
        let kbs = [kb("at-translated-set-2-keyboard", false, false, false), kb("emobie-inject", true, false, true)];
        assert_eq!(plan(&kbs), Some((false, false)));
    }

    #[test]
    fn missing_device_retries() {
        let kbs = [kb("at-translated-set-2-keyboard", true, false, true)];
        assert_eq!(plan(&kbs), None);
    }

    #[test]
    fn suffixed_names_are_ours() {
        assert!(is_ours("emobie-inject"));
        assert!(is_ours("emobie-inject-1"));
        assert!(!is_ours("emobie-injector"));
        assert!(!is_ours("emobie-inject-"));
    }
}
