use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::thread;
use std::time::Duration;

const KEY_GAP: Duration = Duration::from_micros(1000);

pub fn can_open_uinput() -> bool {
    std::path::Path::new("/dev/uinput").exists()
        || std::path::Path::new("/dev/input/uinput").exists()
}

/// True when paste/inject is plausible in this session (compositor and/or uinput).
pub fn can_inject() -> bool {
    can_open_uinput()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("DISPLAY").is_some()
}

fn with_enigo<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&mut Enigo) -> Result<T, String>,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        f(&mut enigo)
    }));
    match result {
        Ok(inner) => inner,
            Err(_) => Err("input injection backend panicked".into()),
    }
}

pub fn inject_ctrl_v() -> Result<(), String> {
    with_enigo(|enigo| {
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| e.to_string())?;
        thread::sleep(KEY_GAP);
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| e.to_string())?;
        thread::sleep(KEY_GAP);
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub fn inject_backspaces(count: usize) -> Result<(), String> {
    with_enigo(|enigo| {
        for _ in 0..count {
            enigo
                .key(Key::Backspace, Direction::Click)
                .map_err(|e| e.to_string())?;
            thread::sleep(KEY_GAP);
        }
        Ok(())
    })
}

pub fn inject_ascii(text: &str) -> Result<(), String> {
    with_enigo(|enigo| {
        enigo.text(text).map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn inject_spaces(count: usize) -> Result<(), String> {
    if count == 0 {
        return Ok(());
    }
    with_enigo(|enigo| {
        for _ in 0..count {
            enigo
                .key(Key::Space, Direction::Click)
                .map_err(|e| e.to_string())?;
            thread::sleep(KEY_GAP);
        }
        Ok(())
    })
}

fn split_trailing_spaces(text: &str) -> (&str, usize) {
    let trimmed = text.trim_end_matches(' ');
    let spaces = text.len() - trimmed.len();
    (trimmed, spaces)
}

pub fn expand_trigger(trigger_chars: usize, expansion: &str) -> Result<(), String> {
    inject_backspaces(trigger_chars)?;
    let (body, trailing_spaces) = split_trailing_spaces(expansion);
    if !body.is_empty() {
        if body.is_ascii() && body.chars().all(|c| !c.is_control() || c == '\n') {
            // Prefer typing ASCII; fall back to clipboard paste for safety on failure.
            if inject_ascii(body).is_err() {
                paste_expansion(body)?;
            }
        } else {
            paste_expansion(body)?;
        }
    }
    inject_spaces(trailing_spaces)
}

fn paste_expansion(expansion: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(expansion)
        .map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(30));
    inject_ctrl_v()?;
    if let Some(prev) = previous {
        thread::sleep(Duration::from_millis(350));
        let _ = clipboard.set_text(prev);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::split_trailing_spaces;

    #[test]
    fn split_trailing_spaces_counts_ascii_spaces() {
        assert_eq!(split_trailing_spaces("hiya"), ("hiya", 0));
        assert_eq!(split_trailing_spaces("hiya "), ("hiya", 1));
        assert_eq!(split_trailing_spaces("hiya  "), ("hiya", 2));
        assert_eq!(split_trailing_spaces(" "), ("", 1));
        assert_eq!(split_trailing_spaces(""), ("", 0));
    }
}
