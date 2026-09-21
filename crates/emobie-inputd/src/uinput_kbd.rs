//! Kernel uinput virtual keyboard — reaches native Wayland apps.
//!
//! Plasma often lacks `zwp_virtual_keyboard_manager_v1`, so Enigo's Wayland path
//! cannot connect. X11/XTest only reaches XWayland clients. A `/dev/uinput`
//! keyboard is seen by the compositor as hardware input and works everywhere
//! the user already granted via `emobie-input` + udev.

use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, BusType, EventType, InputEvent, InputId, Key};
use std::thread;
use std::time::Duration;

const KEY_GAP: Duration = Duration::from_millis(2);
const POST_CREATE_DELAY: Duration = Duration::from_millis(80);
const DEVICE_NAME: &str = "emobie-inject";

pub struct UInputKeyboard {
    device: evdev::uinput::VirtualDevice,
}

impl UInputKeyboard {
    pub fn open() -> Result<Self, String> {
        let mut keys = AttributeSet::<Key>::new();
        for key in [
            Key::KEY_LEFTCTRL,
            Key::KEY_RIGHTCTRL,
            Key::KEY_LEFTSHIFT,
            Key::KEY_RIGHTSHIFT,
            Key::KEY_LEFTALT,
            Key::KEY_RIGHTALT,
            Key::KEY_LEFTMETA,
            Key::KEY_F12,
            Key::KEY_V,
            Key::KEY_C,
            Key::KEY_INSERT,
            Key::KEY_BACKSPACE,
            Key::KEY_ENTER,
            Key::KEY_SPACE,
            Key::KEY_TAB,
            Key::KEY_ESC,
            Key::KEY_GRAVE,
            Key::KEY_MINUS,
            Key::KEY_EQUAL,
            Key::KEY_LEFTBRACE,
            Key::KEY_RIGHTBRACE,
            Key::KEY_BACKSLASH,
            Key::KEY_SEMICOLON,
            Key::KEY_APOSTROPHE,
            Key::KEY_COMMA,
            Key::KEY_DOT,
            Key::KEY_SLASH,
            Key::KEY_A,
            Key::KEY_B,
            Key::KEY_C,
            Key::KEY_D,
            Key::KEY_E,
            Key::KEY_F,
            Key::KEY_G,
            Key::KEY_H,
            Key::KEY_I,
            Key::KEY_J,
            Key::KEY_K,
            Key::KEY_L,
            Key::KEY_M,
            Key::KEY_N,
            Key::KEY_O,
            Key::KEY_P,
            Key::KEY_Q,
            Key::KEY_R,
            Key::KEY_S,
            Key::KEY_T,
            Key::KEY_U,
            Key::KEY_V,
            Key::KEY_W,
            Key::KEY_X,
            Key::KEY_Y,
            Key::KEY_Z,
            Key::KEY_1,
            Key::KEY_2,
            Key::KEY_3,
            Key::KEY_4,
            Key::KEY_5,
            Key::KEY_6,
            Key::KEY_7,
            Key::KEY_8,
            Key::KEY_9,
            Key::KEY_0,
        ] {
            keys.insert(key);
        }

        let device = VirtualDeviceBuilder::new()
            .map_err(|e| format!("uinput open: {e}"))?
            .name(DEVICE_NAME)
            .input_id(InputId::new(BusType::BUS_USB, 0x2e6f, 0x696e, 1))
            .with_keys(&keys)
            .map_err(|e| format!("uinput keys: {e}"))?
            .build()
            .map_err(|e| format!("uinput create: {e}"))?;

        // Compositor needs a beat to pick up the new device before first events.
        thread::sleep(POST_CREATE_DELAY);

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

    /// Tap `key`, optionally holding LeftShift.
    pub fn tap(&mut self, key: Key, shift: bool) -> Result<(), String> {
        if shift {
            self.emit_key(Key::KEY_LEFTSHIFT, 1)?;
        }
        let typed = self.click(key);
        let released = if shift {
            self.emit_key(Key::KEY_LEFTSHIFT, 0)
        } else {
            Ok(())
        };
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
