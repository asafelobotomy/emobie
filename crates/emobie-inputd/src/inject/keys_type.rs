//! ASCII key-typing via uinput (Espanso-style path for short expansions).

use evdev::Key;

use crate::uinput_kbd::UInputKeyboard;

/// Max length for key-safe expansions (skip clipboard).
pub const KEY_SAFE_MAX_CHARS: usize = 48;

/// True when expansion should use clipboard/EI instead of raw keys.
pub fn needs_clipboard_insert(body: &str) -> bool {
    let mut chars = 0usize;
    for c in body.chars() {
        if c == '\n' || c == '\r' || c == '\t' || !c.is_ascii() {
            return true;
        }
        if !is_typeable_ascii(c) {
            return true;
        }
        chars += 1;
        if chars > KEY_SAFE_MAX_CHARS {
            return true;
        }
    }
    false
}

/// Enigo path: prefer literal/clipboard for unsafe bodies (legacy name).
pub fn prefers_literal_insert(body: &str) -> bool {
    needs_clipboard_insert(body)
}

fn is_typeable_ascii(c: char) -> bool {
    matches!(
        c,
        'a'..='z'
            | 'A'..='Z'
            | '0'..='9'
            | ' '
            | '.'
            | ','
            | '-'
            | '_'
            | '='
            | '/'
            | '\\'
            | ';'
            | '\''
            | '['
            | ']'
            | '`'
            | '!'
            | '?'
            | ':'
            | '+'
            | '*'
            | '('
            | ')'
    )
}

/// Map a typeable ASCII char to (key, needs_shift). Layout-sensitive symbols that
/// differ across US/GB are excluded by `is_typeable_ascii`.
fn key_for_char(c: char) -> Option<(Key, bool)> {
    let lower = c.to_ascii_lowercase();
    let letter = match lower {
        'a' => Some(Key::KEY_A),
        'b' => Some(Key::KEY_B),
        'c' => Some(Key::KEY_C),
        'd' => Some(Key::KEY_D),
        'e' => Some(Key::KEY_E),
        'f' => Some(Key::KEY_F),
        'g' => Some(Key::KEY_G),
        'h' => Some(Key::KEY_H),
        'i' => Some(Key::KEY_I),
        'j' => Some(Key::KEY_J),
        'k' => Some(Key::KEY_K),
        'l' => Some(Key::KEY_L),
        'm' => Some(Key::KEY_M),
        'n' => Some(Key::KEY_N),
        'o' => Some(Key::KEY_O),
        'p' => Some(Key::KEY_P),
        'q' => Some(Key::KEY_Q),
        'r' => Some(Key::KEY_R),
        's' => Some(Key::KEY_S),
        't' => Some(Key::KEY_T),
        'u' => Some(Key::KEY_U),
        'v' => Some(Key::KEY_V),
        'w' => Some(Key::KEY_W),
        'x' => Some(Key::KEY_X),
        'y' => Some(Key::KEY_Y),
        'z' => Some(Key::KEY_Z),
        _ => None,
    };
    if let Some(key) = letter {
        return Some((key, c.is_ascii_uppercase()));
    }
    Some(match c {
        ' ' => (Key::KEY_SPACE, false),
        '0' => (Key::KEY_0, false),
        '1' => (Key::KEY_1, false),
        '2' => (Key::KEY_2, false),
        '3' => (Key::KEY_3, false),
        '4' => (Key::KEY_4, false),
        '5' => (Key::KEY_5, false),
        '6' => (Key::KEY_6, false),
        '7' => (Key::KEY_7, false),
        '8' => (Key::KEY_8, false),
        '9' => (Key::KEY_9, false),
        '.' => (Key::KEY_DOT, false),
        ',' => (Key::KEY_COMMA, false),
        '-' => (Key::KEY_MINUS, false),
        '_' => (Key::KEY_MINUS, true),
        '=' => (Key::KEY_EQUAL, false),
        '+' => (Key::KEY_EQUAL, true),
        '/' => (Key::KEY_SLASH, false),
        '?' => (Key::KEY_SLASH, true),
        ';' => (Key::KEY_SEMICOLON, false),
        ':' => (Key::KEY_SEMICOLON, true),
        '\'' => (Key::KEY_APOSTROPHE, false),
        '!' => (Key::KEY_1, true),
        '*' => (Key::KEY_8, true),
        '(' => (Key::KEY_9, true),
        ')' => (Key::KEY_0, true),
        '`' => (Key::KEY_GRAVE, false),
        '[' => (Key::KEY_LEFTBRACE, false),
        ']' => (Key::KEY_RIGHTBRACE, false),
        '\\' => (Key::KEY_BACKSLASH, false),
        _ => return None,
    })
}

pub fn type_string(kbd: &mut UInputKeyboard, body: &str) -> Result<(), String> {
    for c in body.chars() {
        let (key, shift) = key_for_char(c)
            .ok_or_else(|| format!("cannot type char {c:?} via uinput keys"))?;
        kbd.tap(key, shift)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{needs_clipboard_insert, KEY_SAFE_MAX_CHARS};

    #[test]
    fn short_ascii_is_key_safe() {
        assert!(!needs_clipboard_insert("hiya"));
        assert!(!needs_clipboard_insert("hello_world"));
        assert!(!needs_clipboard_insert("path/to-file.txt"));
    }

    #[test]
    fn complex_needs_clipboard() {
        assert!(needs_clipboard_insert("line1\nline2"));
        assert!(needs_clipboard_insert("😀"));
        assert!(needs_clipboard_insert(&"a".repeat(KEY_SAFE_MAX_CHARS + 1)));
        assert!(needs_clipboard_insert("email@x.com")); // @ not in safe set
    }
}
