//! Key ↔ char mapping for the session layout, via libxkbcommon.
//!
//! One shared state serves every keyboard device (so Shift on one keyboard and
//! a letter on another still combine) and the typer, which needs the same
//! layout to turn text back into key presses.
//!
//! Reload only when the layout fingerprint changes, and never while keys are
//! held — rebuilding `xkb_state` mid-chord drops Shift/Caps and mis-maps.

mod sources;
pub mod watch;

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use xkbcommon::xkb::{self, KeyDirection, Keycode, KEYMAP_COMPILE_NO_FLAGS};

const EVDEV_XKB_OFFSET: u32 = 8;
const PERIODIC_RELOAD_SECS: u64 = 30;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_RIGHTALT: u16 = 100;
/// Highest evdev code the virtual keyboard registers.
pub const MAX_TYPED_CODE: u16 = 248;
/// Keypad keys: apps (terminals in application-keypad mode) treat them
/// differently from the main row, so never type through them.
const KEYPAD_CODES: &[u16] = &[
    55, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 96, 98, 117, 118, 121, 179, 180,
];
/// Modifier combinations tried to reach a character, cheapest first.
const COMBOS: [&[u16]; 4] = [&[], &[KEY_LEFTSHIFT], &[KEY_RIGHTALT], &[KEY_LEFTSHIFT, KEY_RIGHTALT]];

/// One character as a key press: `mods` are held around `code` (evdev codes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPlan {
    pub code: u16,
    pub mods: &'static [u16],
}

/// libxkbcommon objects are not thread-safe (non-atomic refcounts), but are
/// fine to move between threads. Every use of this state — and of keymaps or
/// scratch states derived from it — happens with the mutex held.
struct XkbState(xkb::State);

// SAFETY: see `XkbState`; all access is serialized by `KeymapState::state`.
unsafe impl Send for XkbState {}

impl std::ops::Deref for XkbState {
    type Target = xkb::State;
    fn deref(&self) -> &xkb::State {
        &self.0
    }
}

impl std::ops::DerefMut for XkbState {
    fn deref_mut(&mut self) -> &mut xkb::State {
        &mut self.0
    }
}

pub struct KeymapState {
    state: Mutex<XkbState>,
    /// Fingerprint of the RMLVO used to build `state`.
    fingerprint: Mutex<String>,
    /// `watch::generation()` the current state was built for.
    seen_generation: AtomicU64,
    /// Unix seconds of the last `reload_periodically` run.
    last_periodic_reload: AtomicU64,
    /// Keys currently down (evdev codes). Reload is deferred while non-empty.
    /// A set, not a counter: autorepeat re-reports a held key as pressed, and a
    /// counter would then never return to zero (blocking layout reloads).
    pressed: Mutex<HashSet<u16>>,
}

pub fn shared() -> &'static KeymapState {
    static SHARED: OnceLock<KeymapState> = OnceLock::new();
    SHARED.get_or_init(|| {
        watch::start();
        KeymapState::new()
    })
}

fn xkb_code(evdev_code: u16) -> Keycode {
    Keycode::new(u32::from(evdev_code) + EVDEV_XKB_OFFSET)
}

impl KeymapState {
    pub fn new() -> Self {
        let generation = watch::generation();
        let (state, fingerprint) = load_session_state(0);
        Self {
            state: Mutex::new(XkbState(state)),
            fingerprint: Mutex::new(fingerprint),
            seen_generation: AtomicU64::new(generation),
            last_periodic_reload: AtomicU64::new(0),
            pressed: Mutex::new(HashSet::new()),
        }
    }

    fn any_key_held(&self) -> bool {
        self.pressed.lock().map(|set| !set.is_empty()).unwrap_or(true)
    }

    /// Reload layout when session config changed and no keys are held.
    /// Returns true if the keymap was replaced.
    pub fn reload_from_session_if_idle(&self) -> bool {
        if self.any_key_held() {
            return false;
        }
        let generation = watch::generation();
        let Ok(locked_mods) = self
            .state
            .lock()
            .map(|g| g.serialize_mods(xkb::STATE_MODS_LOCKED))
        else {
            return false;
        };
        // Resolving the layout spawns gsettings — do it without the state
        // lock so key handling and typing never wait on it.
        let (state, fingerprint) = load_session_state(locked_mods);
        let Ok(mut fp) = self.fingerprint.lock() else {
            return false;
        };
        self.seen_generation.store(generation, Ordering::Release);
        if *fp == fingerprint {
            return false;
        }
        let Ok(mut guard) = self.state.lock() else {
            return false;
        };
        // Re-check pressed after taking locks — a key may have gone down.
        if self.any_key_held() {
            return false;
        }
        *guard = XkbState(state);
        *fp = fingerprint;
        true
    }

    /// Catch-all for layout changes no watcher reports; shared by every
    /// device thread so it runs once per interval, not once per device.
    pub fn reload_periodically(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let last = self.last_periodic_reload.load(Ordering::Acquire);
        if now.saturating_sub(last) < PERIODIC_RELOAD_SECS {
            return;
        }
        if self
            .last_periodic_reload
            .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let _ = self.reload_from_session_if_idle();
        }
    }

    /// Cheap per-event check: reload only after a layout-switch notification.
    pub fn reload_if_layout_changed(&self) {
        if watch::generation() != self.seen_generation.load(Ordering::Acquire) {
            let _ = self.reload_from_session_if_idle();
        }
    }

    pub fn update_key(&self, evdev_code: u16, pressed: bool) {
        let direction = if pressed {
            KeyDirection::Down
        } else {
            KeyDirection::Up
        };
        // Take and drop the `pressed` lock before `state`: the reload path holds
        // `state` while checking `pressed`.
        let transition = match self.pressed.lock() {
            // insert/remove report whether the key actually changed state.
            Ok(mut set) if pressed => set.insert(evdev_code),
            Ok(mut set) => set.remove(&evdev_code),
            Err(_) => false,
        };
        // Autorepeat re-reports held keys (modifiers too) as pressed. xkb counts
        // every Down, so one Up would leave e.g. Alt latched on forever after
        // Alt+Tab — only feed it real transitions.
        if !transition {
            return;
        }
        if let Ok(mut guard) = self.state.lock() {
            guard.update_key(xkb_code(evdev_code), direction);
        }
    }

    /// True while Ctrl, Alt or Super is held (AltGr / Level3 is not a shortcut).
    pub fn shortcut_modifier_held(&self) -> bool {
        let Ok(guard) = self.state.lock() else {
            return false;
        };
        [xkb::MOD_NAME_CTRL, xkb::MOD_NAME_ALT, xkb::MOD_NAME_LOGO]
            .iter()
            .any(|name| guard.mod_name_is_active(name, xkb::STATE_MODS_EFFECTIVE))
    }

    /// UTF-8 produced by the active layout for an initial key press.
    pub fn key_utf8(&self, evdev_code: u16) -> Option<String> {
        let guard = self.state.lock().ok()?;
        let text = guard.key_get_utf8(xkb_code(evdev_code));
        (!text.is_empty()).then_some(text)
    }

    /// Which key (plus Shift / AltGr) types `c` in the active layout, honouring
    /// the current Caps Lock state. `None` when the layout cannot type it
    /// directly (dead-key or Compose sequences, emoji, other scripts).
    pub fn plan_char(&self, c: char) -> Option<KeyPlan> {
        // Newline/Tab have keys, but typing Enter submits forms and chats.
        if c.is_control() {
            return None;
        }
        let guard = self.state.lock().ok()?;
        let keymap = guard.get_keymap();
        let locked_mods = guard.serialize_mods(xkb::STATE_MODS_LOCKED);
        let group = guard.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE);
        let forbidden = [xkb::MOD_NAME_ALT, xkb::MOD_NAME_CTRL, xkb::MOD_NAME_LOGO]
            .iter()
            .map(|name| keymap.mod_get_index(name))
            .filter(|idx| *idx != xkb::MOD_INVALID)
            .fold(0u32, |mask, idx| mask | (1 << idx));
        let min = keymap.min_keycode().raw().max(EVDEV_XKB_OFFSET + 1);
        let max = keymap
            .max_keycode()
            .raw()
            .min(u32::from(MAX_TYPED_CODE) + EVDEV_XKB_OFFSET);
        let target = c as u32;
        for mods in COMBOS {
            let mut scratch = xkb::State::new(&keymap);
            scratch.update_mask(0, 0, locked_mods, 0, 0, group);
            for m in mods {
                scratch.update_key(xkb_code(*m), KeyDirection::Down);
            }
            // e.g. Right Alt is plain Alt on US: holding it would turn the
            // key into a shortcut rather than a character.
            if scratch.serialize_mods(xkb::STATE_MODS_DEPRESSED) & forbidden != 0 {
                continue;
            }
            for raw in min..=max {
                let code = (raw - EVDEV_XKB_OFFSET) as u16;
                if KEYPAD_CODES.contains(&code) || mods.contains(&code) {
                    continue;
                }
                if scratch.key_get_utf32(Keycode::new(raw)) == target {
                    return Some(KeyPlan { code, mods });
                }
            }
        }
        None
    }
}

impl Default for KeymapState {
    fn default() -> Self {
        Self::new()
    }
}

fn compile(ctx: &xkb::Context, rmlvo: &sources::Rmlvo) -> Option<xkb::Keymap> {
    let or = |value: &str, fallback: &str| {
        if value.is_empty() {
            fallback.to_string()
        } else {
            value.to_string()
        }
    };
    xkb::Keymap::new_from_names(
        ctx,
        &or(&rmlvo.rules, "evdev"),
        &or(&rmlvo.model, "pc105"),
        &rmlvo.layout,
        &rmlvo.variant,
        (!rmlvo.options.is_empty()).then(|| rmlvo.options.clone()),
        KEYMAP_COMPILE_NO_FLAGS,
    )
}

/// Build a state for the session layout with its active group locked and
/// `locked_mods` (Caps/Num Lock) carried over.
fn load_session_state(locked_mods: u32) -> (xkb::State, String) {
    let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let rmlvo = sources::resolve(watch::plasma_group());
    let keymap = compile(&ctx, &rmlvo).unwrap_or_else(|| {
        xkb::Keymap::new_from_names(&ctx, "evdev", "pc105", "us", "", None, KEYMAP_COMPILE_NO_FLAGS)
            .expect("fallback us keymap")
    });
    let mut state = xkb::State::new(&keymap);
    state.update_mask(0, 0, locked_mods, 0, 0, rmlvo.group);
    (state, rmlvo.fingerprint())
}

#[cfg(test)]
mod tests;
