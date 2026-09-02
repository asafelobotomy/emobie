use ::enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::thread;
use std::time::Duration;

use super::clipboard::{
    ensure_clipboard_text, prefers_literal_insert, restore_clipboard_now,
    schedule_clipboard_restore, set_clipboard_text,
};
use crate::session_env;

/// Only used when expand fires before the completing key is released (overlap).
const PRE_ERASE_DELAY: Duration = Duration::from_millis(12);
/// Brief settle so the focused app applies backspaces before insert.
const POST_ERASE_DELAY: Duration = Duration::from_millis(8);
const KEY_GAP: Duration = Duration::from_millis(1);
/// Let the focused app process Ctrl+V before we drop listen suppress.
pub(super) const POST_PASTE_DELAY: Duration = Duration::from_millis(40);
/// Recreate Enigo after idle — Wayland virtual-keyboard seats go stale.
pub(super) const ENIGO_MAX_IDLE: Duration = Duration::from_secs(45);

pub(super) fn new_enigo() -> Result<Enigo, String> {
    catch_unwind(AssertUnwindSafe(|| {
        let mut settings = Settings::default();
        // Re-detect every time: systemd often starts us before Wayland exists.
        // Read-only — do not call ensure_session_env from worker threads.
        settings.wayland_display = session_env::wayland_display_for_enigo();
        Enigo::new(&settings)
    }))
    .map_err(|_| "input injection backend panicked".to_string())?
    .map_err(|e| e.to_string())
}

/// Map paste chord keys into the virtual keymap and let the compositor adopt it.
/// Without this, the first Unicode('v') regenerates the keymap mid-chord and the
/// Click is dropped — trigger is erased, expansion never appears; retry works.
pub(super) fn warm_up_enigo(enigo: &mut Enigo) {
    for c in ['v', 'V', 'c', 'C'] {
        let _ = enigo.key(Key::Unicode(c), Direction::Release);
    }
    let _ = enigo.key(Key::Control, Direction::Release);
    let _ = enigo.key(Key::Shift, Direction::Release);
    let _ = enigo.key(Key::Alt, Direction::Release);
    let _ = enigo.key(Key::Meta, Direction::Release);
    thread::sleep(Duration::from_millis(40));
}

fn click_key(enigo: &mut Enigo, key: Key) -> Result<(), String> {
    enigo
        .key(key, Direction::Click)
        .map_err(|e| e.to_string())?;
    thread::sleep(KEY_GAP);
    Ok(())
}

fn ensure_paste_key_mapped(enigo: &mut Enigo) {
    // Release-only maps the keysym + applies keymap without inserting a character.
    let _ = enigo.key(Key::Unicode('v'), Direction::Release);
    thread::sleep(Duration::from_millis(15));
}

pub(super) fn ctrl_v_enigo(enigo: &mut Enigo) -> Result<(), String> {
    ensure_paste_key_mapped(enigo);
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    let typed = (|| -> Result<(), String> {
        thread::sleep(KEY_GAP);
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| e.to_string())?;
        thread::sleep(KEY_GAP);
        Ok(())
    })();
    // Always release Control — a stuck modifier poisons the session.
    let released = enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string());
    typed.and(released)
}

fn erase_chars(enigo: &mut Enigo, count: usize) -> Result<(), String> {
    let count = count.min(crate::state::MAX_TRIGGER_LEN);
    for _ in 0..count {
        click_key(enigo, Key::Backspace)?;
    }
    if count > 0 {
        thread::sleep(POST_ERASE_DELAY);
    }
    Ok(())
}

fn paste_with_enigo(enigo: &mut Enigo, body: &str) -> Result<(), String> {
    let epoch = set_clipboard_text(body)?;
    ctrl_v_enigo(enigo)?;
    thread::sleep(POST_PASTE_DELAY);
    schedule_clipboard_restore(body.to_string(), epoch);
    Ok(())
}

pub(super) fn retype_trigger_enigo(enigo: &mut Enigo, trigger: &str) {
    if trigger.is_empty() {
        return;
    }
    if prefers_literal_insert(trigger) {
        let _ = paste_with_enigo(enigo, trigger);
    } else {
        let _ = enigo.text(trigger);
    }
}

pub(super) fn expand_with_enigo(
    enigo: &mut Enigo,
    trigger_chars: usize,
    expansion: &str,
    trigger: &str,
    trigger_committed: bool,
) -> Result<(), String> {
    if !trigger_committed {
        thread::sleep(PRE_ERASE_DELAY);
    }

    // Clipboard first so erase failure can restore it; paste failure can retype trigger.
    if expansion.contains('\0') {
        return Err("expansion contains NUL".into());
    }
    let epoch = if expansion.is_empty() {
        None
    } else {
        Some(set_clipboard_text(expansion)?)
    };

    if let Err(err) = erase_chars(enigo, trigger_chars) {
        if epoch.is_some() {
            restore_clipboard_now();
        }
        return Err(err);
    }

    if expansion.is_empty() {
        return Ok(());
    }

    if let Err(err) = ensure_clipboard_text(expansion) {
        retype_trigger_enigo(enigo, trigger);
        restore_clipboard_now();
        return Err(err);
    }

    match ctrl_v_enigo(enigo) {
        Ok(()) => {
            // Give the focused app time to handle paste before suppress ends.
            thread::sleep(POST_PASTE_DELAY);
            if let Some(epoch) = epoch {
                schedule_clipboard_restore(expansion.to_string(), epoch);
            }
            Ok(())
        }
        Err(paste_err) => {
            retype_trigger_enigo(enigo, trigger);
            restore_clipboard_now();
            if !prefers_literal_insert(expansion) && enigo.text(expansion).is_ok() {
                return Ok(());
            }
            Err(paste_err)
        }
    }
}
