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

/// Async so it runs off the main thread: applying the pin can shell out and
/// (on GNOME) wait for window focus, which round-trips to the main thread.
#[tauri::command]
pub async fn apply_window_pin(app: AppHandle, pinned: bool) -> Result<PinApplyResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(window) = app.get_webview_window("main") else {
            return Err("main window missing".to_string());
        };
        Ok(apply_to_window(&window, pinned))
    })
    .await
    .map_err(|err| err.to_string())?
}

/// Async: probing the compositor shells out (gsettings / qdbus via flatpak-spawn).
#[tauri::command]
pub async fn pin_capability() -> PinCapability {
    tauri::async_runtime::spawn_blocking(pin_capability_blocking)
        .await
        .unwrap_or_else(|err| PinCapability {
            wayland: false,
            plasma: false,
            reliable: false,
            detail: format!("Could not check pin support ({err})."),
            gnome_setup_needed: false,
        })
}

fn pin_capability_blocking() -> PinCapability {
    #[cfg(target_os = "linux")]
    {
        linux::capability()
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
pub async fn pin_gnome_setup() -> Result<PinCapability, String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(target_os = "linux")]
        {
            linux::gnome::setup_binding()?;
            Ok(linux::capability())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err("GNOME pin setup is Linux-only.".into())
        }
    })
    .await
    .map_err(|err| format!("pin setup task failed: {err}"))?
}

pub fn apply_to_window(window: &WebviewWindow, pinned: bool) -> PinApplyResult {
    let _ = window.set_always_on_top(pinned);
    #[cfg(target_os = "linux")]
    {
        linux::apply_compositor_pin(window, pinned)
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
