//! GNOME Wayland pin via Mutter's own unused `toggle-above` window
//! keybinding (`org.gnome.desktop.wm.keybindings`) — no custom Shell
//! extension needed. Mutter never exposes `make_above`/`unmake_above` to
//! external callers directly, but it does let a keybinding toggle it, and
//! that binding ships unbound by default. We claim it (only if still empty —
//! never overwriting a user's own shortcut) and have emobie-inputd
//! synthesize the exact keypress via `/dev/uinput`.
//!
//! Because Mutter *toggles* rather than *sets* the state, correctness here
//! depends on tracking what we believe the real state is (`LAST_ABOVE`) and
//! only sending a keypress on an actual transition — see `toggle_pin` and
//! `note_hidden`.

use crate::pin::PinApplyResult;
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;

/// Must match `UInputKeyboard::toggle_above_gnome` /
/// `toggle_above_gnome_enigo` in emobie-inputd exactly.
const TOGGLE_ABOVE_BINDING: &str = "<Control><Alt><Super>F12";
const SCHEMA: &str = "org.gnome.desktop.wm.keybindings";
const KEY: &str = "toggle-above";

const STATE_UNKNOWN: u8 = 0;
const STATE_BELOW: u8 = 1;
const STATE_ABOVE: u8 = 2;

/// Our best guess at whether the window is currently "above" — reset to
/// unknown on hide (see `note_hidden`) so a real re-toggle happens on next
/// show rather than being skipped because this still remembers the old value.
static LAST_ABOVE: AtomicU8 = AtomicU8::new(STATE_UNKNOWN);
/// Serializes the whole check → inject → record sequence in `toggle_pin`.
/// Mutter *toggles*, so two overlapping callers (tray show + startup, rapid
/// activations) would both see "not yet pinned" and toggle twice.
static TOGGLE_LOCK: Mutex<()> = Mutex::new(());
/// Bumped by `note_hidden`; lets an in-flight toggle notice the window was
/// hidden underneath it and not record a stale state.
static HIDE_EPOCH: AtomicU64 = AtomicU64::new(0);

pub fn desktop_is_gnome() -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    desktop
        .split(':')
        .any(|part| part.eq_ignore_ascii_case("GNOME"))
}

pub enum BindingStatus {
    NotGnome,
    Unconfigured,
    Ready,
    Conflict(String),
}

pub fn binding_status() -> BindingStatus {
    if !desktop_is_gnome() {
        return BindingStatus::NotGnome;
    }
    match current_binding() {
        Some(bindings) if bindings.is_empty() => BindingStatus::Unconfigured,
        Some(bindings) if bindings.iter().any(|b| b == TOGGLE_ABOVE_BINDING) => {
            BindingStatus::Ready
        }
        Some(bindings) => BindingStatus::Conflict(bindings.join(", ")),
        None => BindingStatus::Conflict("could not read the GNOME shortcut".into()),
    }
}

/// Only ever writes when the binding is currently empty — never overwrites
/// an existing shortcut the user (or another app) already set.
pub fn setup_binding() -> Result<(), String> {
    match binding_status() {
        BindingStatus::NotGnome => Err("Not running under GNOME.".into()),
        BindingStatus::Ready => Ok(()),
        BindingStatus::Conflict(existing) => Err(format!(
            "GNOME's toggle-above shortcut is already bound to {existing} — not overwriting it."
        )),
        BindingStatus::Unconfigured => {
            let value = format!("['{TOGGLE_ABOVE_BINDING}']");
            gsettings_call(&["set", SCHEMA, KEY, &value])?;
            Ok(())
        }
    }
}

/// Reset tracked state when the window is hidden. Whether or not Mutter
/// actually clears "above" on hide (unverified for GNOME specifically — the
/// existing Plasma path assumes WMs commonly do this), resetting to unknown
/// guarantees the next show-while-pinned always re-asserts for real instead
/// of silently no-op'ing because our tracker still says "already above".
pub fn note_hidden() {
    HIDE_EPOCH.fetch_add(1, Ordering::AcqRel);
    LAST_ABOVE.store(STATE_UNKNOWN, Ordering::Relaxed);
}

/// True when `toggle_pin(pinned)` would do nothing (state already as requested),
/// so callers can skip waiting for window focus.
pub fn is_noop(pinned: bool) -> bool {
    let desired = if pinned { STATE_ABOVE } else { STATE_BELOW };
    already_in_state(LAST_ABOVE.load(Ordering::Relaxed), desired)
}

/// Whether `toggle_pin(desired)` would be a no-op given what we believe.
/// An unknown state counts as "not above": a freshly mapped window is never
/// above, so a request to *unpin* must not send the (toggling) chord — that
/// would pin it instead.
fn already_in_state(last: u8, desired: u8) -> bool {
    last == desired || (desired == STATE_BELOW && last == STATE_UNKNOWN)
}

pub fn toggle_pin(pinned: bool) -> PinApplyResult {
    let desired = if pinned { STATE_ABOVE } else { STATE_BELOW };
    let _guard = TOGGLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = HIDE_EPOCH.load(Ordering::Acquire);
    if already_in_state(LAST_ABOVE.load(Ordering::Relaxed), desired) {
        return PinApplyResult {
            applied: true,
            limited: false,
            detail: if pinned {
                "Already pinned.".into()
            } else {
                "Already unpinned.".into()
            },
        };
    }
    match crate::input_helper::unix::inject_pin_toggle() {
        Ok(()) => {
            // Hidden while the chord was in flight → we no longer know.
            let recorded = if HIDE_EPOCH.load(Ordering::Acquire) == epoch {
                desired
            } else {
                STATE_UNKNOWN
            };
            LAST_ABOVE.store(recorded, Ordering::Relaxed);
            PinApplyResult {
                applied: true,
                limited: false,
                detail: if pinned {
                    "Pinned via GNOME toggle-above.".into()
                } else {
                    "Unpinned.".into()
                },
            }
        }
        Err(err) => PinApplyResult {
            applied: false,
            limited: true,
            detail: format!("Could not toggle pin ({err})."),
        },
    }
}

fn current_binding() -> Option<Vec<String>> {
    parse_string_array(&gsettings_call(&["get", SCHEMA, KEY]).ok()?)
}

/// Parses gsettings' text form for an `as` (array of strings) value, e.g.
/// `@as []`, `[]`, or `['<Control><Alt><Super>F12']`.
fn parse_string_array(raw: &str) -> Option<Vec<String>> {
    let trimmed = raw.trim();
    let bracketed = trimmed.strip_prefix("@as ").unwrap_or(trimmed).trim();
    let inner = bracketed.strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    Some(
        inner
            .split(',')
            .map(|part| part.trim().trim_matches('\'').to_string())
            .collect(),
    )
}

fn gsettings_call(args: &[&str]) -> Result<String, String> {
    // Flatpak: gsettings must reach the host's dconf, not the sandbox's own
    // isolated config — same flatpak-spawn --host pattern as the KWin qdbus calls.
    if super::in_flatpak() {
        let output = Command::new("flatpak-spawn")
            .arg("--host")
            .arg("gsettings")
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        return finish(output);
    }
    let output = Command::new("gsettings")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    finish(output)
}

fn finish(output: std::process::Output) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_state_never_sends_unpin_chord() {
        assert!(already_in_state(STATE_UNKNOWN, STATE_BELOW));
        assert!(already_in_state(STATE_BELOW, STATE_BELOW));
        assert!(!already_in_state(STATE_ABOVE, STATE_BELOW));
    }

    #[test]
    fn unknown_state_still_pins() {
        assert!(!already_in_state(STATE_UNKNOWN, STATE_ABOVE));
        assert!(!already_in_state(STATE_BELOW, STATE_ABOVE));
        assert!(already_in_state(STATE_ABOVE, STATE_ABOVE));
    }

    #[test]
    fn parses_gsettings_arrays() {
        assert_eq!(parse_string_array("@as []"), Some(vec![]));
        assert_eq!(
            parse_string_array("['<Control><Alt><Super>F12']"),
            Some(vec!["<Control><Alt><Super>F12".to_string()])
        );
    }
}
