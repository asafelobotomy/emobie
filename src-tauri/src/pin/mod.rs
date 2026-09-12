//! Always-on-top / pin helpers.
//!
//! GTK `set_keep_above` works on X11. On Wayland it is often a no-op; on Plasma
//! we set KWin `keepAbove` via a short script (host `qdbus` under Flatpak). On
//! GNOME Wayland there is no external API for this at all (Mutter only exposes
//! `make_above`/`unmake_above` to code running inside the Shell process) — see
//! `linux::gnome` for the workaround: binding Mutter's own unused `toggle-above`
//! keybinding to a fixed chord, then synthesizing that keypress via emobie-inputd.

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinApplyResult {
    pub applied: bool,
    /// True when the compositor may ignore keep-above (typical non-Plasma Wayland).
    pub limited: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinCapability {
    pub wayland: bool,
    pub plasma: bool,
    pub reliable: bool,
    pub detail: String,
    /// True on GNOME Wayland when the `toggle-above` shortcut isn't set up
    /// yet — Settings can offer a one-time, consent-gated setup button.
    #[serde(default)]
    pub gnome_setup_needed: bool,
}

#[tauri::command]
pub fn apply_window_pin(app: AppHandle, pinned: bool) -> Result<PinApplyResult, String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("main window missing".into());
    };
    Ok(apply_to_window(&window, pinned))
}

#[tauri::command]
pub fn pin_capability() -> PinCapability {
    #[cfg(target_os = "linux")]
    {
        return linux::capability();
    }
    #[cfg(not(target_os = "linux"))]
    {
        PinCapability {
            wayland: false,
            plasma: false,
            reliable: true,
            detail: "Pin uses the native always-on-top API.".into(),
            gnome_setup_needed: false,
        }
    }
}

/// One-time, consent-gated: binds GNOME's unused `toggle-above` keybinding to
/// the fixed chord emobie-inputd sends. Never overwrites an existing binding —
/// see `linux::gnome::setup_binding`.
#[tauri::command]
pub fn pin_gnome_setup() -> Result<PinCapability, String> {
    #[cfg(target_os = "linux")]
    {
        linux::gnome::setup_binding()?;
        return Ok(linux::capability());
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("GNOME pin setup is Linux-only.".into())
    }
}

pub fn apply_to_window(window: &WebviewWindow, pinned: bool) -> PinApplyResult {
    let _ = window.set_always_on_top(pinned);
    #[cfg(target_os = "linux")]
    {
        return linux::apply_compositor_pin(pinned);
    }
    #[cfg(not(target_os = "linux"))]
    {
        PinApplyResult {
            applied: true,
            limited: false,
            detail: if pinned {
                "Pinned above other windows.".into()
            } else {
                "Unpinned.".into()
            },
        }
    }
}

pub fn apply_from_prefs(window: &WebviewWindow) {
    let _ = apply_to_window(window, crate::prefs::pinned());
}

/// Call whenever the main window is hidden (tray hide, close-to-tray). Only
/// meaningful on GNOME's toggle-based path — see `linux::gnome::note_hidden`.
pub fn note_window_hidden() {
    #[cfg(target_os = "linux")]
    {
        linux::gnome::note_hidden();
    }
}

#[cfg(target_os = "linux")]
mod linux;
