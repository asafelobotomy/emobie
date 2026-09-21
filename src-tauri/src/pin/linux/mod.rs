use super::{PinApplyResult, PinCapability};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::WebviewWindow;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub(super) mod gnome;

pub(super) fn in_flatpak() -> bool {
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

    if wayland && !plasma && gnome::desktop_is_gnome() {
        return match gnome::binding_status() {
            gnome::BindingStatus::Ready => PinCapability {
                wayland,
                plasma,
                reliable: true,
                detail: "Pin uses a GNOME toggle-above shortcut.".into(),
                gnome_setup_needed: false,
            },
            gnome::BindingStatus::Unconfigured => PinCapability {
                wayland,
                plasma,
                reliable: false,
                detail: "Pin needs a one-time shortcut setup on GNOME Wayland.".into(),
                gnome_setup_needed: true,
            },
            gnome::BindingStatus::Conflict(existing) => PinCapability {
                wayland,
                plasma,
                reliable: false,
                detail: format!(
                    "GNOME's toggle-above shortcut is already bound to {existing} — \
emobie will not override it. Use GNOME's own Super+right-click window menu instead."
                ),
                gnome_setup_needed: false,
            },
            gnome::BindingStatus::NotGnome => unreachable!("checked desktop_is_gnome above"),
        };
    }

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
        gnome_setup_needed: false,
    }
}

/// The GNOME toggle is a synthetic keypress that lands on whatever window has
/// focus, so wait (bounded) for *our* window to be focused before sending it.
/// Must not be called on the main thread — `is_focused` round-trips to it.
fn wait_for_focus(window: &WebviewWindow) -> bool {
    for _ in 0..30 {
        if window.is_focused().unwrap_or(false) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

pub fn apply_compositor_pin(window: &WebviewWindow, pinned: bool) -> PinApplyResult {
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

    // Unconfigured/conflict falls through to the same "limited" result
    // Plasma-less Wayland already returns below, so the user still gets an
    // honest explanation instead of silent failure.
    if gnome::desktop_is_gnome()
        && !desktop_is_plasma()
        && matches!(gnome::binding_status(), gnome::BindingStatus::Ready)
    {
        // Already in the requested state: nothing will be sent, so no focus needed.
        if !gnome::is_noop(pinned) && !wait_for_focus(window) {
            return PinApplyResult {
                applied: false,
                limited: false,
                detail: "emobie is not focused yet — the pin is applied when it gains focus."
                    .into(),
            };
        }
        return gnome::toggle_pin(pinned);
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

    // Only the per-user runtime dir: a predictable file under a shared /tmp
    // could be pre-created or symlinked by another user.
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "XDG_RUNTIME_DIR is not set".to_string())?
        .join("emobie");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("kwin-pin.js");
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| e.to_string())?;
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
