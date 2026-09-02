use std::thread;
use std::time::Duration;

use super::clipboard::{
    restore_clipboard_now, schedule_clipboard_restore, set_clipboard_text,
};
use super::enigo::POST_PASTE_DELAY;
use crate::uinput_kbd::UInputKeyboard;

/// Only used when expand fires before the completing key is released (overlap).
const PRE_ERASE_DELAY: Duration = Duration::from_millis(12);

fn paste_with_uinput(kbd: &mut UInputKeyboard, body: &str) -> Result<(), String> {
    let epoch = set_clipboard_text(body)?;
    kbd.ctrl_v()?;
    thread::sleep(POST_PASTE_DELAY);
    schedule_clipboard_restore(body.to_string(), epoch);
    Ok(())
}

pub(super) fn retype_trigger_uinput(kbd: &mut UInputKeyboard, trigger: &str) {
    if trigger.is_empty() {
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
    let epoch = if expansion.is_empty() {
        None
    } else {
        Some(set_clipboard_text(expansion)?)
    };

    let erase = trigger_chars.min(crate::state::MAX_TRIGGER_LEN);
    if let Err(err) = kbd.erase_chars(erase) {
        if epoch.is_some() {
            restore_clipboard_now();
        }
        return Err(err);
    }

    if expansion.is_empty() {
        return Ok(());
    }

    match kbd.ctrl_v() {
        Ok(()) => {
            thread::sleep(POST_PASTE_DELAY);
            if let Some(epoch) = epoch {
                schedule_clipboard_restore(expansion.to_string(), epoch);
            }
            Ok(())
        }
        Err(paste_err) => {
            retype_trigger_uinput(kbd, trigger);
            restore_clipboard_now();
            Err(paste_err)
        }
    }
}
