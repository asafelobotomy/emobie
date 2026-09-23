//! Layout-aware key typing via uinput (Espanso-style path, no clipboard).
//!
//! The compositor maps our key codes through the session layout, so text is
//! planned against that same layout (`crate::keymap`) rather than a fixed
//! US table. Anything the layout cannot type goes through the clipboard.

use crate::keymap::{self, KeyPlan};
use crate::uinput_kbd::UInputKeyboard;

/// Longer bodies paste instead: typing costs roughly 5 ms per character.
pub const TYPED_MAX_CHARS: usize = 160;
/// Enigo types its own way; keep its old conservative ASCII limit.
const ENIGO_LITERAL_MAX_CHARS: usize = 48;

/// Key presses for `body`, or `None` when any char needs the clipboard.
pub fn plan_text(body: &str) -> Option<Vec<KeyPlan>> {
    if body.chars().count() > TYPED_MAX_CHARS {
        return None;
    }
    let keymap = keymap::shared();
    body.chars().map(|c| keymap.plan_char(c)).collect()
}

pub fn type_plan(kbd: &mut UInputKeyboard, plan: &[KeyPlan]) -> Result<(), String> {
    for key in plan {
        kbd.type_key(key)?;
    }
    Ok(())
}

/// Enigo path: true when the body should be pasted rather than typed.
pub fn prefers_literal_insert(body: &str) -> bool {
    let mut chars = 0usize;
    for c in body.chars() {
        if !is_enigo_typeable(c) {
            return true;
        }
        chars += 1;
        if chars > ENIGO_LITERAL_MAX_CHARS {
            return true;
        }
    }
    false
}

fn is_enigo_typeable(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            ' ' | '.' | ',' | '-' | '_' | '=' | '/' | '\\' | ';' | '\'' | '[' | ']' | '`' | '!'
                | '?' | ':' | '+' | '*' | '(' | ')'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_and_long_text_need_clipboard() {
        assert!(plan_text("line1\nline2").is_none());
        assert!(plan_text("a\tb").is_none());
        assert!(plan_text("😀").is_none());
        assert!(plan_text(&"a".repeat(TYPED_MAX_CHARS + 1)).is_none());
    }

    #[test]
    fn plain_words_are_typed() {
        let plan = plan_text("hiya").expect("latin letters exist on the test layout");
        assert_eq!(plan.len(), 4);
    }

    #[test]
    fn enigo_literal_rules_unchanged() {
        assert!(!prefers_literal_insert("hello_world"));
        assert!(prefers_literal_insert("email@x.com"));
        assert!(prefers_literal_insert("line1\nline2"));
    }
}
