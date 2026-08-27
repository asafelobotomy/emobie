//! Always-on-top / pin helpers.
//!
//! GTK `set_keep_above` is a no-op on many Wayland compositors. On Plasma we
//! also set KWin `keepAbove` via a short script so pin works on Wayland.

use tauri::{AppHandle, Manager, WebviewWindow};

#[tauri::command]
pub fn apply_window_pin(app: AppHandle, pinned: bool) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("main window missing".into());
    };
    apply_to_window(&window, pinned);
    Ok(())
}

pub fn apply_to_window(window: &WebviewWindow, pinned: bool) {
    let _ = window.set_always_on_top(pinned);
    #[cfg(target_os = "linux")]
    linux::apply_compositor_pin(pinned);
}

pub fn apply_from_prefs(window: &WebviewWindow) {
    apply_to_window(window, crate::prefs::pinned());
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    pub fn apply_compositor_pin(pinned: bool) {
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return;
        }
        // Window may not be mapped yet right after show(); retry once.
        if plasma_keep_above(pinned).is_err() && pinned {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(150));
                let _ = plasma_keep_above(true);
            });
        }
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

        let plugin = "emobie-pin";
        let path_str = path.to_string_lossy().to_string();
        let _ = qdbus(&[
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.unloadScript",
            plugin,
        ]);
        let id = qdbus(&[
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.loadScript",
            &path_str,
            plugin,
        ])?;
        let script_path = format!("/Scripting/Script{id}");
        qdbus(&["org.kde.KWin", &script_path, "org.kde.kwin.Script.run"])?;
        let _ = qdbus(&[
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.unloadScript",
            plugin,
        ]);
        Ok(())
    }

    fn qdbus(args: &[&str]) -> Result<String, String> {
        for bin in ["qdbus6", "qdbus"] {
            let output = Command::new(bin).args(args).output();
            match output {
                Ok(out) if out.status.success() => {
                    return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
                }
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
        Err("qdbus unavailable".into())
    }
}
