//! Focused-window-aware paste chord selection.
//!
//! Ctrl+V is the right chord for most GUI apps, but many terminal emulators
//! bind paste to Ctrl+Shift+V instead — Ctrl+V is claimed by the shell's
//! readline (`^V` = quoted-insert), confirmed live: emobie-inputd's Ctrl+V
//! delivered zero bytes to a focused GNOME Console window, while Ctrl+Shift+V
//! pasted correctly. No single global chord is safe for every app: Kate
//! binds Ctrl+Shift+V to "Switch to Next Input Mode" (a real KDE default,
//! not a hypothetical), so blindly adding it back would silently change the
//! editor's input mode instead of double-pasting. See docs/MACROS.md "Known
//! limitations" for the full history.
//!
//! `decide` never errors and never blocks — an unknown or undetectable app
//! (`focused_class: None`) always falls back to the pre-existing Ctrl+V
//! default, matching behavior before this module existed.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteChord {
    CtrlV,
    ShiftInsert,
    CtrlShiftV,
}

const OVERRIDE_AUTO: u8 = 0;
const OVERRIDE_CTRL_V: u8 = 1;
const OVERRIDE_SHIFT_INSERT: u8 = 2;
const OVERRIDE_CTRL_SHIFT_V: u8 = 3;

/// User-facing manual override (Settings), bypasses detection entirely when set.
static USER_OVERRIDE: AtomicU8 = AtomicU8::new(OVERRIDE_AUTO);

/// Accepts "auto" (default), "ctrl_v", "shift_insert", "ctrl_shift_v". Any
/// other value resets to "auto" rather than erroring — a stale/misspelled
/// pref should never wedge Auto-paste, it should just fall back to guessing.
pub fn set_override(value: &str) {
    let code = match value {
        "ctrl_v" => OVERRIDE_CTRL_V,
        "shift_insert" => OVERRIDE_SHIFT_INSERT,
        "ctrl_shift_v" => OVERRIDE_CTRL_SHIFT_V,
        _ => OVERRIDE_AUTO,
    };
    USER_OVERRIDE.store(code, Ordering::Relaxed);
}

pub fn override_label() -> &'static str {
    match USER_OVERRIDE.load(Ordering::Relaxed) {
        OVERRIDE_CTRL_V => "ctrl_v",
        OVERRIDE_SHIFT_INSERT => "shift_insert",
        OVERRIDE_CTRL_SHIFT_V => "ctrl_shift_v",
        _ => "auto",
    }
}

/// Apps that bind Ctrl+Shift+V to something other than paste — never send it
/// there even if they're mistakenly matched as a terminal-like app.
/// Kate/KWrite: KDE's default "Switch to Next Input Mode" shortcut.
const AVOID_CTRL_SHIFT_V: &[&str] = &["kate", "org.kde.kate", "kwrite", "org.kde.kwrite"];

/// Known terminal emulators (WM_CLASS / app-id, lowercased) that need
/// Ctrl+Shift+V instead of Ctrl+V. Not exhaustive — matches the "curated
/// compatibility list" approach Espanso uses for the same problem, since a
/// fully generic rule doesn't exist (see module docs).
const TERMINAL_CLASSES: &[&str] = &[
    "gnome-terminal",
    "gnome-terminal-server",
    "org.gnome.terminal",
    "gnome-console",
    "org.gnome.console",
    "kgx",
    "konsole",
    "org.kde.konsole",
    "xterm",
    "uxterm",
    "urxvt",
    "rxvt",
    "alacritty",
    "kitty",
    "foot",
    "wezterm",
    "tilix",
    "terminator",
    "xfce4-terminal",
    "lxterminal",
    "mate-terminal",
    "terminology",
    "ghostty",
];

/// `focused_class` is a best-effort WM_CLASS/app-id from `focused_window`, or
/// `None` when detection failed or wasn't available — always safe to pass
/// either way.
pub fn decide(focused_class: Option<&str>) -> PasteChord {
    match USER_OVERRIDE.load(Ordering::Relaxed) {
        OVERRIDE_CTRL_V => return PasteChord::CtrlV,
        OVERRIDE_SHIFT_INSERT => return PasteChord::ShiftInsert,
        OVERRIDE_CTRL_SHIFT_V => return PasteChord::CtrlShiftV,
        _ => {}
    }
    let Some(class) = focused_class else {
        return PasteChord::CtrlV;
    };
    let class = class.to_lowercase();
    if AVOID_CTRL_SHIFT_V.iter().any(|known| *known == class) {
        return PasteChord::CtrlV;
    }
    if TERMINAL_CLASSES.iter().any(|known| *known == class) {
        return PasteChord::CtrlShiftV;
    }
    PasteChord::CtrlV
}

#[cfg(test)]
mod tests {
    use super::*;

    // A single test: USER_OVERRIDE is process-global, and cargo runs tests in
    // parallel threads by default — splitting override-mutating assertions
    // across separate #[test] fns would race with each other.
    #[test]
    fn decide_behavior() {
        set_override("auto");
        assert_eq!(decide(None), PasteChord::CtrlV);
        assert_eq!(decide(Some("some-random-app")), PasteChord::CtrlV);

        assert_eq!(decide(Some("Gnome-console")), PasteChord::CtrlShiftV);
        assert_eq!(decide(Some("org.gnome.Console")), PasteChord::CtrlShiftV);
        assert_eq!(decide(Some("konsole")), PasteChord::CtrlShiftV);

        assert_eq!(decide(Some("kate")), PasteChord::CtrlV);
        assert_eq!(decide(Some("org.kde.kate")), PasteChord::CtrlV);

        set_override("shift_insert");
        assert_eq!(decide(Some("konsole")), PasteChord::ShiftInsert);
        assert_eq!(decide(None), PasteChord::ShiftInsert);

        set_override("auto");
        assert_eq!(decide(Some("konsole")), PasteChord::CtrlShiftV);
    }
}
