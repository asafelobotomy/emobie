//! Key-event handling for the match buffer and pending expands.

use evdev::Key;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::inject;
use crate::keymap::KeymapState;
use crate::matcher::TriggerTrie;

pub(super) struct PendingExpand {
    pub erase: usize,
    pub expansion: String,
    pub trigger: String,
    pub key_code: u16,
    pub created_at: Instant,
}

/// Completing key held longer than this → cancel (lost release / stuck key).
const PENDING_TIMEOUT: Duration = Duration::from_secs(3);

fn is_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::KEY_LEFTSHIFT
            | Key::KEY_RIGHTSHIFT
            | Key::KEY_LEFTCTRL
            | Key::KEY_RIGHTCTRL
            | Key::KEY_LEFTALT
            | Key::KEY_RIGHTALT
            | Key::KEY_LEFTMETA
            | Key::KEY_RIGHTMETA
            | Key::KEY_CAPSLOCK
            | Key::KEY_NUMLOCK
            | Key::KEY_SCROLLLOCK
            | Key::KEY_FN
    )
}

/// Mouse, touchpad and touchscreen presses move the caret or focus.
fn is_pointer_button(key: Key) -> bool {
    matches!(
        key,
        Key::BTN_LEFT | Key::BTN_RIGHT | Key::BTN_MIDDLE | Key::BTN_SIDE | Key::BTN_EXTRA | Key::BTN_TOUCH
    )
}

/// Keyboard keys live below `BTN_MISC`; codes above are buttons and touchpad
/// tool reports (`BTN_TOOL_FINGER`…), which must never touch the buffer.
const FIRST_BUTTON_CODE: u16 = 0x100;
/// Like libinput's disable-while-typing: a palm resting on the touchpad while
/// typing reports raw touches the desktop ignores. Only a touch after a pause
/// in typing is a real tap that may have moved the caret.
const TOUCH_AFTER_TYPING_GRACE_MS: u64 = 1_000;
static LAST_KEY_PRESS_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// True when a pointer press should reset the typed buffer.
fn pointer_press_moves_caret(key: Key, now: u64, last_key_press: u64) -> bool {
    key != Key::BTN_TOUCH || now.saturating_sub(last_key_press) >= TOUCH_AFTER_TYPING_GRACE_MS
}

/// `EMOBIE_INPUTD_DEBUG=1`: log why the buffer resets and when triggers match.
/// Never logs typed text — only reasons and lengths.
fn debug(msg: impl FnOnce() -> String) {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("EMOBIE_INPUTD_DEBUG").is_some()) {
        eprintln!("emobie-inputd[debug]: {}", msg());
    }
}

/// The caret may have moved: forget typed text and any pending expand.
fn reset_buffer(pending: &Mutex<Option<PendingExpand>>, buffer: &Mutex<String>, reason: &str) {
    debug(|| {
        let held = buffer.lock().map(|b| b.chars().count()).unwrap_or(0);
        format!("buffer reset ({reason}, {held} chars dropped)")
    });
    if let Ok(mut guard) = pending.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = buffer.lock() {
        guard.clear();
    }
}

fn is_edit_or_nav(key: Key) -> bool {
    matches!(
        key,
        Key::KEY_BACKSPACE
            | Key::KEY_DELETE
            | Key::KEY_ENTER
            | Key::KEY_TAB
            | Key::KEY_ESC
            | Key::KEY_LEFT
            | Key::KEY_RIGHT
            | Key::KEY_UP
            | Key::KEY_DOWN
            | Key::KEY_HOME
            | Key::KEY_END
            | Key::KEY_PAGEUP
            | Key::KEY_PAGEDOWN
    )
}

pub(super) fn trim_buffer(guard: &mut String, max_buf: usize) {
    let count = guard.chars().count();
    if count <= max_buf {
        return;
    }
    let skip = count - max_buf;
    *guard = guard.chars().skip(skip).collect();
}

fn fire_expand(
    pending: PendingExpand,
    trigger_committed: bool,
    enabled: &AtomicBool,
    buffer: &Mutex<String>,
) {
    if !enabled.load(Ordering::Relaxed) {
        // Leave the trigger on screen; put it back in the buffer.
        if let Ok(mut guard) = buffer.lock() {
            guard.push_str(&pending.trigger);
            trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
        }
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
            trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
        }
    }
}

/// Cancel a pending expand and restore its trigger into the match buffer.
fn cancel_pending(pending: &Mutex<Option<PendingExpand>>, buffer: &Mutex<String>) {
    let cancelled = {
        let Ok(mut guard) = pending.lock() else {
            return;
        };
        guard.take()
    };
    if let Some(p) = cancelled {
        if let Ok(mut guard) = buffer.lock() {
            guard.push_str(&p.trigger);
            trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
        }
    }
}

/// Drop pending expands whose completing-key release never arrived.
pub(super) fn expire_stale_pending(pending: &Mutex<Option<PendingExpand>>, buffer: &Mutex<String>) {
    let stale = {
        let Ok(mut guard) = pending.lock() else {
            return;
        };
        let is_stale = guard
            .as_ref()
            .is_some_and(|p| p.created_at.elapsed() >= PENDING_TIMEOUT);
        if is_stale {
            guard.take()
        } else {
            None
        }
    };
    if let Some(p) = stale {
        if let Ok(mut guard) = buffer.lock() {
            guard.push_str(&p.trigger);
            trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
        }
    }
}

pub(super) fn handle_key(
    key: Key,
    value: i32,
    keymap: &KeymapState,
    enabled: &AtomicBool,
    buffer: &Mutex<String>,
    trie: &Mutex<TriggerTrie>,
    pending: &Mutex<Option<PendingExpand>>,
) {
    if is_pointer_button(key) {
        if value == 1
            && pointer_press_moves_caret(key, now_ms(), LAST_KEY_PRESS_MS.load(Ordering::Relaxed))
        {
            reset_buffer(pending, buffer, "pointer press");
        }
        return;
    }
    if key.code() >= FIRST_BUTTON_CODE {
        return;
    }
    if value == 1 {
        LAST_KEY_PRESS_MS.store(now_ms(), Ordering::Relaxed);
    }
    let pressed = value != 0;
    keymap.update_key(key.code(), pressed);
    // Keys typed on the lock screen are the user's password.
    if crate::guard::session_locked() {
        reset_buffer(pending, buffer, "session locked");
        return;
    }

    // Completing-key release must fire even while inject suppress is active.
    // Otherwise a concurrent emoji paste can swallow the release and leave
    // pending stuck until another key (or timeout).
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
            fire_expand(p, true, enabled, buffer);
        }
        return;
    }

    // Synthetic keys from our own inject — keep keymap in sync, ignore buffer.
    if inject::should_suppress_keys() {
        return;
    }

    // Only initial presses update the match buffer (ignore autorepeat).
    if value != 1 {
        return;
    }

    // Modifiers while waiting for completing-key release must not flush expand.
    if is_modifier(key) {
        return;
    }

    // Edit/nav while pending: cancel expand (user is revising), restore trigger,
    // then apply the edit to the buffer so it stays aligned with the app.
    if is_edit_or_nav(key) {
        cancel_pending(pending, buffer);
    } else {
        // Printable (or other) key while pending: flush expand first so the next
        // character cannot race ahead of the replacement, then fall through and
        // buffer this key so the match buffer stays aligned with the app.
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
    // Ctrl/Alt/Super + key is a shortcut (paste, undo, workspace switch…):
    // whatever it did, the buffer no longer mirrors the text before the caret.
    if keymap.shortcut_modifier_held() {
        reset_buffer(pending, buffer, "shortcut modifier");
        return;
    }
    if key == Key::KEY_BACKSPACE {
        if let Ok(mut guard) = buffer.lock() {
            guard.pop();
        }
        return;
    }
    if key == Key::KEY_ENTER || key == Key::KEY_TAB || key == Key::KEY_ESC {
        if let Ok(mut guard) = buffer.lock() {
            guard.clear();
        }
        return;
    }
    if is_edit_or_nav(key) {
        // Arrows / Home / End / Delete: buffer no longer matches caret — reset.
        if let Ok(mut guard) = buffer.lock() {
            guard.clear();
        }
        return;
    }

    let Some(text) = keymap.key_utf8(key.code()) else {
        return;
    };
    // Dead-key first press yields empty (handled above). Compose may yield one
    // or more non-control chars — push them all then match once.
    let produced: Vec<char> = text.chars().filter(|c| !c.is_control()).collect();
    if produced.is_empty() {
        return;
    }

    let hit = {
        let Ok(mut guard) = buffer.lock() else {
            return;
        };
        for ch in produced {
            guard.push(ch);
        }
        trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
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
        debug(|| format!("trigger matched ({len} chars), waiting for key release"));
        if let Ok(mut guard) = pending.lock() {
            *guard = Some(PendingExpand {
                erase: len,
                expansion,
                trigger,
                key_code: key.code(),
                created_at: Instant::now(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palm_touch_while_typing_keeps_buffer() {
        assert!(!pointer_press_moves_caret(Key::BTN_TOUCH, 10_500, 10_000));
        assert!(pointer_press_moves_caret(Key::BTN_TOUCH, 12_000, 10_000));
    }

    #[test]
    fn clicks_always_reset() {
        assert!(pointer_press_moves_caret(Key::BTN_LEFT, 10_100, 10_000));
        assert!(pointer_press_moves_caret(Key::BTN_RIGHT, 10_100, 10_000));
    }

    #[test]
    fn touchpad_tool_codes_are_not_keyboard_keys() {
        assert!(Key::BTN_TOOL_FINGER.code() >= FIRST_BUTTON_CODE);
        assert!(Key::BTN_TOUCH.code() >= FIRST_BUTTON_CODE);
        assert!(Key::KEY_MICMUTE.code() < FIRST_BUTTON_CODE);
    }
}
