//! Always-on-top / pin helpers.
//!
//! GTK `set_keep_above` works on X11. On Wayland it is often a no-op; on Plasma
//! we set KWin `keepAbove` via a short script (host `qdbus` under Flatpak).

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
        }
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

#[cfg(target_os = "linux")]
mod linux {
    use super::{PinApplyResult, PinCapability};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    fn in_flatpak() -> bool {
        std::env::var_os("FLATPAK_ID").is_some()
    }

    fn on_wayland() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some()
    }

    fn desktop_is_plasma() -> bool {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        desktop
            .split(':')
            .any(|part| part.eq_ignore_ascii_case("KDE") || part.eq_ignore_ascii_case("Plasma"))
    }

    pub fn capability() -> PinCapability {
        let wayland = on_wayland();
        let plasma = desktop_is_plasma() || kwin_reachable();
        let reliable = !wayland || plasma;
        let detail = if !wayland {
            "Pin uses always-on-top (X11).".into()
        } else if plasma {
            "Pin uses KWin keep-above on Plasma Wayland.".into()
        } else {
            "Pin may not stay above other windows on this Wayland compositor \
(works on X11 and Plasma). Use X11 session or Plasma for reliable pin."
                .into()
        };
        PinCapability {
            wayland,
            plasma,
            reliable,
            detail,
        }
    }

    pub fn apply_compositor_pin(pinned: bool) -> PinApplyResult {
        if !on_wayland() {
            return PinApplyResult {
                applied: true,
                limited: false,
                detail: if pinned {
                    "Pinned above other windows.".into()
                } else {
                    "Unpinned.".into()
                },
            };
        }

        match plasma_keep_above(pinned) {
            Ok(()) => PinApplyResult {
                applied: true,
                limited: false,
                detail: if pinned {
                    "Pinned via KWin keep-above.".into()
                } else {
                    "Unpinned.".into()
                },
            },
            Err(err) => {
                if pinned {
                    // Retry once after map (common right after show).
                    let err2 = {
                        thread::sleep(Duration::from_millis(150));
                        plasma_keep_above(true)
                    };
                    if err2.is_ok() {
                        return PinApplyResult {
                            applied: true,
                            limited: false,
                            detail: "Pinned via KWin keep-above.".into(),
                        };
                    }
                    PinApplyResult {
                        applied: false,
                        limited: true,
                        detail: format!(
                            "Pin may not stay above on this Wayland compositor ({err}). \
Works on X11 and Plasma."
                        ),
                    }
                } else {
                    PinApplyResult {
                        applied: true,
                        limited: false,
                        detail: "Unpinned.".into(),
                    }
                }
            }
        }
    }

    fn kwin_reachable() -> bool {
        dbus_call(&[
            "org.kde.KWin",
            "/KWin",
            "org.freedesktop.DBus.Peer.Ping",
        ])
        .is_ok()
    }

    fn plasma_keep_above(pinned: bool) -> Result<(), String> {
        let pid = std::process::id();
        let keep = if pinned { "true" } else { "false" };
        let script = format!(
            r#"const wins = workspace.windowList();
for (const w of wins) {{
  if (w.pid === {pid}) {{
    w.keepAbove = {keep};
  }}
}}
"#
        );

        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("emobie");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("kwin-pin.js");
        {
            let mut file = fs::File::create(&path).map_err(|e| e.to_string())?;
            file.write_all(script.as_bytes())
                .map_err(|e| e.to_string())?;
        }

        // Flatpak sandbox: script must be readable by host KWin — use a path under
        // XDG_RUNTIME_DIR which is shared; invoke qdbus on the host.
        let plugin = "emobie-pin";
        let path_str = path.to_string_lossy().to_string();
        let _ = dbus_call(&[
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.unloadScript",
            plugin,
        ]);
        let id = dbus_call(&[
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.loadScript",
            &path_str,
            plugin,
        ])?;
        let script_path = format!("/Scripting/Script{id}");
        dbus_call(&["org.kde.KWin", &script_path, "org.kde.kwin.Script.run"])?;
        let _ = dbus_call(&[
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.unloadScript",
            plugin,
        ]);
        Ok(())
    }

    fn dbus_call(args: &[&str]) -> Result<String, String> {
        // Prefer host tools inside Flatpak (GNOME Platform rarely ships qdbus).
        if in_flatpak() {
            for bin in ["qdbus6", "qdbus"] {
                let output = Command::new("flatpak-spawn")
                    .arg("--host")
                    .arg(bin)
                    .args(args)
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
                    }
                }
            }
        }
        for bin in ["qdbus6", "qdbus"] {
            let output = Command::new(bin).args(args).output();
            match output {
                Ok(out) if out.status.success() => {
                    return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
                }
                _ => continue,
            }
        }
        Err("qdbus unavailable (install qt6-tools / qdbus; Flatpak uses host via flatpak-spawn)".into())
    }
}
