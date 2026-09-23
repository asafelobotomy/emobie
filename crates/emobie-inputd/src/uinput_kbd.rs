//! Kernel uinput virtual keyboard — reaches native Wayland apps.
//!
//! Plasma often lacks `zwp_virtual_keyboard_manager_v1`, so Enigo's Wayland path
//! cannot connect. X11/XTest only reaches XWayland clients. A `/dev/uinput`
//! keyboard is seen by the compositor as hardware input and works everywhere
//! the user already granted via `emobie-input` + udev.

use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, BusType, EventType, InputEvent, InputId, Key};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

const KEY_GAP: Duration = Duration::from_millis(2);
/// Compositors (libinput's udev backend) only add a device once udev has
/// finished processing it, which is marked by its `/run/udev/data` entry.
const UDEV_INIT_TIMEOUT: Duration = Duration::from_millis(800);
const POST_INIT_SETTLE: Duration = Duration::from_millis(60);
const POST_CREATE_FALLBACK: Duration = Duration::from_millis(300);
pub const DEVICE_NAME: &str = "emobie-inject";
pub const VENDOR_ID: u16 = 0x2e6f;
pub const PRODUCT_ID: u16 = 0x696e;

/// evdev char devices are major 13 with minors starting at 64 for `eventN`.
fn udev_data_path(dev_node: &Path) -> Option<PathBuf> {
    let name = dev_node.file_name()?.to_str()?;
    let index: u32 = name.strip_prefix("event")?.parse().ok()?;
    Some(PathBuf::from(format!("/run/udev/data/c13:{}", 64 + index)))
}

fn wait_for_udev_init(device: &mut evdev::uinput::VirtualDevice) -> bool {
    let deadline = Instant::now() + UDEV_INIT_TIMEOUT;
    while Instant::now() < deadline {
        let marker = device
            .enumerate_dev_nodes_blocking()
            .ok()
            .and_then(|mut nodes| nodes.find_map(Result::ok))
            .and_then(|node| udev_data_path(&node));
        if marker.is_some_and(|path| path.exists()) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

pub struct UInputKeyboard {
    device: evdev::uinput::VirtualDevice,
}

impl UInputKeyboard {
    pub fn open() -> Result<Self, String> {
        // Every key the layout-aware typer may choose (see crate::keymap).
        let mut keys = AttributeSet::<Key>::new();
        for code in 1..=crate::keymap::MAX_TYPED_CODE {
            keys.insert(Key::new(code));
        }

        let mut device = VirtualDeviceBuilder::new()
            .map_err(|e| format!("uinput open: {e}"))?
            .name(DEVICE_NAME)
            .input_id(InputId::new(BusType::BUS_USB, VENDOR_ID, PRODUCT_ID, 1))
            .with_keys(&keys)
            .map_err(|e| format!("uinput keys: {e}"))?
            .build()
            .map_err(|e| format!("uinput create: {e}"))?;

        // Events sent before the compositor has added the device are dropped
        // silently — fatal for GNOME's toggle-above chord, whose state we track.
        if wait_for_udev_init(&mut device) {
            thread::sleep(POST_INIT_SETTLE);
        } else {
            thread::sleep(POST_CREATE_FALLBACK);
        }

        Ok(Self { device })
    }

    fn emit_key(&mut self, key: Key, value: i32) -> Result<(), String> {
        let ev = InputEvent::new(EventType::KEY, key.code(), value);
        self.device
            .emit(&[ev])
            .map_err(|e| format!("uinput emit: {e}"))?;
        thread::sleep(KEY_GAP);
        Ok(())
    }

    pub fn click(&mut self, key: Key) -> Result<(), String> {
        self.emit_key(key, 1)?;
        self.emit_key(key, 0)?;
        Ok(())
    }

    /// Press `plan.mods`, tap `plan.code`, release the mods in reverse.
    pub fn type_key(&mut self, plan: &crate::keymap::KeyPlan) -> Result<(), String> {
        for m in plan.mods {
            self.emit_key(Key::new(*m), 1)?;
        }
        let typed = self.click(Key::new(plan.code));
        let mut released = Ok(());
        for m in plan.mods.iter().rev() {
            released = released.and(self.emit_key(Key::new(*m), 0));
        }
        typed.and(released)
    }

    pub fn erase_chars(&mut self, count: usize) -> Result<(), String> {
        for _ in 0..count {
            self.click(Key::KEY_BACKSPACE)?;
        }
        if count > 0 {
            thread::sleep(Duration::from_millis(8));
        }
        Ok(())
    }

    pub fn ctrl_v(&mut self) -> Result<(), String> {
        self.emit_key(Key::KEY_LEFTCTRL, 1)?;
        let typed = self.click(Key::KEY_V);
        let released = self.emit_key(Key::KEY_LEFTCTRL, 0);
        typed.and(released)
    }

    fn shift_insert(&mut self) -> Result<(), String> {
        self.emit_key(Key::KEY_LEFTSHIFT, 1)?;
        let typed = self.click(Key::KEY_INSERT);
        let released = self.emit_key(Key::KEY_LEFTSHIFT, 0);
        typed.and(released)
    }

    fn ctrl_shift_v(&mut self) -> Result<(), String> {
        self.emit_key(Key::KEY_LEFTCTRL, 1)?;
        self.emit_key(Key::KEY_LEFTSHIFT, 1)?;
        let typed = self.click(Key::KEY_V);
        let shift_released = self.emit_key(Key::KEY_LEFTSHIFT, 0);
        let ctrl_released = self.emit_key(Key::KEY_LEFTCTRL, 0);
        typed.and(shift_released).and(ctrl_released)
    }

    /// Send whichever chord `crate::paste_chord::decide` picked for the
    /// currently focused app.
    pub fn paste_chord(&mut self, chord: crate::paste_chord::PasteChord) -> Result<(), String> {
        use crate::paste_chord::PasteChord;
        match chord {
            PasteChord::CtrlV => self.ctrl_v(),
            PasteChord::ShiftInsert => self.shift_insert(),
            PasteChord::CtrlShiftV => self.ctrl_shift_v(),
        }
    }

    /// Ctrl+Alt+Super+F12 — must match the accelerator string
    /// `src-tauri/src/pin/linux/gnome.rs` writes to GNOME's `toggle-above` keybinding
    /// (`org.gnome.desktop.wm.keybindings`). Toggles whichever window
    /// currently has focus between "always above" and normal — see
    /// `crate::inject::inject_pin_toggle` for why this must only be sent
    /// while the emobie window itself is known to be focused.
    pub fn toggle_above_gnome(&mut self) -> Result<(), String> {
        self.emit_key(Key::KEY_LEFTCTRL, 1)?;
        self.emit_key(Key::KEY_LEFTALT, 1)?;
        self.emit_key(Key::KEY_LEFTMETA, 1)?;
        let typed = self.click(Key::KEY_F12);
        let meta_released = self.emit_key(Key::KEY_LEFTMETA, 0);
        let alt_released = self.emit_key(Key::KEY_LEFTALT, 0);
        let ctrl_released = self.emit_key(Key::KEY_LEFTCTRL, 0);
        typed.and(meta_released).and(alt_released).and(ctrl_released)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_event_node_to_udev_data() {
        assert_eq!(
            udev_data_path(Path::new("/dev/input/event7")),
            Some(PathBuf::from("/run/udev/data/c13:71"))
        );
        assert_eq!(udev_data_path(Path::new("/dev/input/mouse0")), None);
    }

    /// Needs write access to /dev/uinput: `cargo test -- --ignored uinput_open_waits`.
    #[test]
    #[ignore]
    fn uinput_open_waits_for_udev() {
        let start = Instant::now();
        let mut kbd = UInputKeyboard::open().expect("open uinput");
        let elapsed = start.elapsed();
        assert!(wait_for_udev_init(&mut kbd.device), "udev never initialized the device");
        eprintln!("uinput open took {elapsed:?}");
        assert!(elapsed < POST_CREATE_FALLBACK, "fell back to the fixed delay");
    }
}
