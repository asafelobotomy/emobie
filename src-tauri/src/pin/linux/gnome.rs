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
use std::sync::atomic::{AtomicU8, Ordering};

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
    LAST_ABOVE.store(STATE_UNKNOWN, Ordering::Relaxed);
}

pub fn toggle_pin(pinned: bool) -> PinApplyResult {
    let desired = if pinned { STATE_ABOVE } else { STATE_BELOW };
    if LAST_ABOVE.load(Ordering::Relaxed) == desired {
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
            LAST_ABOVE.store(desired, Ordering::Relaxed);
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
