use std::thread;
use std::time::Duration;

use super::clipboard::{
    ensure_clipboard_text, note_backend, restore_clipboard_now, schedule_clipboard_restore,
    set_clipboard_text,
};
use super::ei;
use super::enigo::POST_PASTE_DELAY;
use super::keys_type::{needs_clipboard_insert, type_string};
use crate::uinput_kbd::UInputKeyboard;

/// Only used when expand fires before the completing key is released (overlap).
const PRE_ERASE_DELAY: Duration = Duration::from_millis(12);
/// Recreate the uinput device after idle — like Enigo's Wayland seats, a
/// long-idle virtual device can go stale (compositor drops it from the seat)
/// with writes still succeeding at the kernel level, so no error ever fires
/// the existing failure-recovery path. Bound the staleness window instead of
/// relying on an error that may never come.
pub(super) const UINPUT_MAX_IDLE: Duration = Duration::from_secs(300);

fn paste_chords(kbd: &mut UInputKeyboard) -> Result<(), String> {
    // Ctrl+V only — Kate and most Qt/KDE apps bind both Ctrl+V and Shift+Insert,
    // so sending both unconditionally double-pastes the expansion in them.
    // Known trade-off: some terminal emulators bind paste to Shift+Insert (or
    // Ctrl+Shift+V) specifically because Ctrl+V is claimed by the shell, so
    // Expand can silently no-op there. Fixing that without reintroducing the
    // double-paste needs focused-window/WM_CLASS detection, which doesn't
    // exist in this codebase yet — don't just add Shift+Insert back. See
    // "Known limitations" in docs/MACROS.md.
    kbd.ctrl_v()?;
    thread::sleep(POST_PASTE_DELAY);
    Ok(())
}

fn paste_with_uinput(kbd: &mut UInputKeyboard, body: &str) -> Result<(), String> {
    let epoch = set_clipboard_text(body)?;
    paste_chords(kbd)?;
    schedule_clipboard_restore(body.to_string(), epoch);
    Ok(())
}

pub(super) fn retype_trigger_uinput(kbd: &mut UInputKeyboard, trigger: &str) {
    if trigger.is_empty() {
        return;
    }
    if !needs_clipboard_insert(trigger) {
        let _ = type_string(kbd, trigger);
        return;
    }
    let _ = paste_with_uinput(kbd, trigger);
}

pub(super) fn expand_with_uinput(
    kbd: &mut UInputKeyboard,
    trigger_chars: usize,
    expansion: &str,
    trigger: &str,
    trigger_committed: bool,
) -> Result<(), String> {
    if !trigger_committed {
        thread::sleep(PRE_ERASE_DELAY);
    }
    if expansion.contains('\0') {
        return Err("expansion contains NUL".into());
    }

    let erase = trigger_chars.min(crate::state::MAX_TRIGGER_LEN);
    kbd.erase_chars(erase)?;

    if expansion.is_empty() {
        return Ok(());
    }

    // Key-safe ASCII: type directly — avoids clipboard races entirely.
    if !needs_clipboard_insert(expansion) {
        type_string(kbd, expansion)?;
        note_backend("keys");
        return Ok(());
    }

    // Complex text: optional EI/eitype, then clipboard paste.
    if ei::try_type_without_clipboard(expansion).is_ok() {
        note_backend("ei");
        return Ok(());
    }

    let epoch = set_clipboard_text(expansion)?;
    if let Err(err) = ensure_clipboard_text(expansion) {
        retype_trigger_uinput(kbd, trigger);
        restore_clipboard_now();
        return Err(err);
    }

    match paste_chords(kbd) {
        Ok(()) => {
            schedule_clipboard_restore(expansion.to_string(), epoch);
            Ok(())
        }
        Err(paste_err) => {
            retype_trigger_uinput(kbd, trigger);
            restore_clipboard_now();
            Err(paste_err)
        }
    }
}
