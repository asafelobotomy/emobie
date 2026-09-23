use std::thread;
use std::time::Duration;

use super::clipboard::{
    ensure_clipboard_text, note_backend, restore_clipboard_now, schedule_clipboard_restore,
    set_clipboard_text,
};
use super::ei;
use super::enigo::POST_PASTE_DELAY;
use super::keys_type::{plan_text, type_plan};
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
    // Best-effort focused-window detection picks the chord (Ctrl+V by
    // default; Ctrl+Shift+V for known terminals; never Ctrl+Shift+V for apps
    // like Kate that bind it to something else) — see crate::paste_chord and
    // "Known limitations" in docs/MACROS.md for why no single fixed chord
    // works for every app.
    let chord =
        crate::paste_chord::decide(crate::focused_window::detect_class().as_deref());
    kbd.paste_chord(chord)?;
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
    if let Some(plan) = plan_text(trigger) {
        let _ = type_plan(kbd, &plan);
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

    // Everything the layout can type goes in as keys — no clipboard races
    // and no per-app paste chord.
    if let Some(plan) = plan_text(expansion) {
        type_plan(kbd, &plan)?;
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
