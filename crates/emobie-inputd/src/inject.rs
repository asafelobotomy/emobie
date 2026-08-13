use enigo::{Direction, Enigo, Key, Keyboard, Settings};
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

pub fn inject_ctrl_v() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
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
}

pub fn inject_backspaces(count: usize) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    for _ in 0..count {
        enigo
            .key(Key::Backspace, Direction::Click)
            .map_err(|e| e.to_string())?;
        thread::sleep(KEY_GAP);
    }
    Ok(())
}

pub fn inject_ascii(text: &str) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.text(text).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn expand_trigger(trigger_chars: usize, expansion: &str) -> Result<(), String> {
    inject_backspaces(trigger_chars)?;
    if expansion.is_ascii() && expansion.chars().all(|c| !c.is_control() || c == '\n') {
        // Prefer typing ASCII; fall back to clipboard paste for safety on failure.
        if inject_ascii(expansion).is_ok() {
            return Ok(());
        }
    }
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
