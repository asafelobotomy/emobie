//! Key → char mapping uses libxkbcommon (session layout via XKB_DEFAULT_*).

use std::sync::Mutex;
use xkbcommon::xkb::{self, KeyDirection, KEYMAP_COMPILE_NO_FLAGS};

const EVDEV_XKB_OFFSET: u32 = 8;

pub struct KeymapState {
    state: Mutex<xkb::State>,
}

impl KeymapState {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(session_state()),
        }
    }

    /// Reload layout from session XKB_DEFAULT_* (handles runtime layout changes).
    pub fn reload_from_session(&self) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = session_state();
        }
    }

    pub fn update_key(&self, evdev_code: u16, pressed: bool) {
        let xkb_code = xkb::Keycode::new(u32::from(evdev_code) + EVDEV_XKB_OFFSET);
        let direction = if pressed {
            KeyDirection::Down
        } else {
            KeyDirection::Up
        };
        if let Ok(mut guard) = self.state.lock() {
            guard.update_key(xkb_code, direction);
        }
    }

    /// UTF-8 produced by the active layout for an initial key press.
    pub fn key_utf8(&self, evdev_code: u16) -> Option<String> {
        let xkb_code = xkb::Keycode::new(u32::from(evdev_code) + EVDEV_XKB_OFFSET);
        let guard = self.state.lock().ok()?;
        let text = guard.key_get_utf8(xkb_code);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

fn session_state() -> xkb::State {
    let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = load_keymap(&ctx).unwrap_or_else(|| fallback_keymap(&ctx));
    xkb::State::new(&keymap)
}

fn load_keymap(ctx: &xkb::Context) -> Option<xkb::Keymap> {
    // Empty RMLVO fields → libxkbcommon reads XKB_DEFAULT_* from the session.
    xkb::Keymap::new_from_names(ctx, "", "", "", "", None, KEYMAP_COMPILE_NO_FLAGS)
}

fn fallback_keymap(ctx: &xkb::Context) -> xkb::Keymap {
    xkb::Keymap::new_from_names(
        ctx,
        "evdev",
        "pc105",
        "us",
        "",
        None,
        KEYMAP_COMPILE_NO_FLAGS,
    )
    .expect("fallback us keymap")
}

#[cfg(test)]
mod tests {
    use super::KeymapState;

    #[test]
    fn keymap_state_maps_ascii_key() {
        let km = KeymapState::new();
        // KEY_A evdev code 30
        km.update_key(30, true);
        let text = km.key_utf8(30).unwrap_or_default();
        assert!(
            text.chars().next().is_some_and(|c| c.is_ascii_alphabetic()),
            "expected a letter, got {text:?}"
        );
        km.update_key(30, false);
    }
}
