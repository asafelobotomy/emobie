use evdev::{Device, InputEventKind, Key};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::inject;
use crate::matcher::TriggerTrie;

fn is_keyboard(device: &Device) -> bool {
    device.supported_keys().is_some_and(|keys| {
        keys.contains(Key::KEY_A) && keys.contains(Key::KEY_Z) && keys.contains(Key::KEY_ENTER)
    })
}

fn list_keyboard_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Ok(entries) = fs::read_dir("/dev/input") else {
        return paths;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("event") {
            continue;
        }
        if let Ok(device) = Device::open(&path) {
            if is_keyboard(&device) {
                paths.push(path);
            }
        }
    }
    paths
}

fn key_to_char(key: Key, shift: bool) -> Option<char> {
    let ch = match key {
        Key::KEY_A => 'a',
        Key::KEY_B => 'b',
        Key::KEY_C => 'c',
        Key::KEY_D => 'd',
        Key::KEY_E => 'e',
        Key::KEY_F => 'f',
        Key::KEY_G => 'g',
        Key::KEY_H => 'h',
        Key::KEY_I => 'i',
        Key::KEY_J => 'j',
        Key::KEY_K => 'k',
        Key::KEY_L => 'l',
        Key::KEY_M => 'm',
        Key::KEY_N => 'n',
        Key::KEY_O => 'o',
        Key::KEY_P => 'p',
        Key::KEY_Q => 'q',
        Key::KEY_R => 'r',
        Key::KEY_S => 's',
        Key::KEY_T => 't',
        Key::KEY_U => 'u',
        Key::KEY_V => 'v',
        Key::KEY_W => 'w',
        Key::KEY_X => 'x',
        Key::KEY_Y => 'y',
        Key::KEY_Z => 'z',
        Key::KEY_1 => '1',
        Key::KEY_2 => '2',
        Key::KEY_3 => '3',
        Key::KEY_4 => '4',
        Key::KEY_5 => '5',
        Key::KEY_6 => '6',
        Key::KEY_7 => '7',
        Key::KEY_8 => '8',
        Key::KEY_9 => '9',
        Key::KEY_0 => '0',
        Key::KEY_MINUS => '-',
        Key::KEY_EQUAL => '=',
        Key::KEY_LEFTBRACE => '[',
        Key::KEY_RIGHTBRACE => ']',
        Key::KEY_SEMICOLON => ';',
        Key::KEY_APOSTROPHE => '\'',
        Key::KEY_GRAVE => '`',
        Key::KEY_BACKSLASH => '\\',
        Key::KEY_COMMA => ',',
        Key::KEY_DOT => '.',
        Key::KEY_SLASH => '/',
        Key::KEY_SPACE => ' ',
        _ => return None,
    };
    if !shift {
        return Some(ch);
    }
    Some(match ch {
        'a'..='z' => (ch as u8 - 32) as char,
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        ';' => ':',
        '\'' => '"',
        '`' => '~',
        '\\' => '|',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        _ => ch,
    })
}

pub fn can_listen() -> bool {
    !list_keyboard_paths().is_empty()
}

pub fn spawn_listener(
    enabled: Arc<AtomicBool>,
    trie: Arc<Mutex<TriggerTrie>>,
    stop: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut buffer = String::new();
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let paths = list_keyboard_paths();
            if paths.is_empty() {
                thread::sleep(Duration::from_secs(2));
                continue;
            }
            let Ok(mut device) = Device::open(&paths[0]) else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            let mut shift = false;
            loop {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                let events = match device.fetch_events() {
                    Ok(events) => events,
                    Err(_) => {
                        thread::sleep(Duration::from_millis(50));
                        break;
                    }
                };
                for event in events {
                    if let InputEventKind::Key(key) = event.kind() {
                        let value = event.value();
                        if key == Key::KEY_LEFTSHIFT || key == Key::KEY_RIGHTSHIFT {
                            shift = value != 0;
                            continue;
                        }
                        if value != 1 {
                            continue;
                        }
                        if !enabled.load(Ordering::Relaxed) {
                            buffer.clear();
                            continue;
                        }
                        if key == Key::KEY_BACKSPACE {
                            buffer.pop();
                            continue;
                        }
                        if key == Key::KEY_ENTER || key == Key::KEY_TAB {
                            buffer.clear();
                            continue;
                        }
                        if let Some(ch) = key_to_char(key, shift) {
                            buffer.push(ch);
                            if buffer.len() > 128 {
                                let trim: String = buffer.chars().skip(buffer.len() - 96).collect();
                                buffer = trim;
                            }
                            let hit = {
                                let guard = trie.lock().unwrap();
                                guard.match_suffix(&buffer)
                            };
                            if let Some((len, expansion)) = hit {
                                let _ = inject::expand_trigger(len, &expansion);
                                for _ in 0..len {
                                    buffer.pop();
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}
