//! Launch-on-startup registration for native installs and Flatpak.
//!
//! Native / GitHub Flatpak (with `xdg-config/autostart`): write a `.desktop` file.
//! Flathub-constrained Flatpak: use the XDG Background portal (no autostart FS).

use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "emobie";
const FLATPAK_APP_ID: &str = "io.github.asafelobotomy.emobie";
const LEGACY_DESKTOP: &str = "emobie.desktop";
const MARKER_NAME: &str = "autostart-enabled";

fn is_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

fn autostart_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".config").join("autostart"))
}

fn desktop_path() -> Result<PathBuf, String> {
    let name = if is_flatpak() {
        format!("{FLATPAK_APP_ID}.desktop")
    } else {
        LEGACY_DESKTOP.to_string()
    };
    Ok(autostart_dir()?.join(name))
}

fn marker_path() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        })?;
    Some(data.join(FLATPAK_APP_ID).join(MARKER_NAME))
}

fn native_autostart_exec() -> Result<String, String> {
    fn escape_desktop_path(path: &PathBuf) -> String {
        path.display().to_string().replace(' ', "\\ ")
    }

    // Prefer stable install paths over the ephemeral AppImage mount at current_exe().
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        let path = PathBuf::from(&appimage);
        if path.is_file() {
            return Ok(escape_desktop_path(&path));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let launcher = PathBuf::from(&home).join(".local/bin/emobie");
        if launcher.is_file() {
            return Ok(escape_desktop_path(&launcher));
        }
        let appimage = PathBuf::from(&home).join(".local/bin/emobie.AppImage");
        if appimage.is_file() {
            return Ok(format!(
                "env APPIMAGE_EXTRACT_AND_RUN=1 {}",
                escape_desktop_path(&appimage)
            ));
        }
    }
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let exe = exe.canonicalize().unwrap_or(exe);
    Ok(escape_desktop_path(&PathBuf::from(exe)))
}

fn native_desktop_contents() -> Result<String, String> {
    let exe = native_autostart_exec()?;
    Ok(format!(
        "\
[Desktop Entry]
Type=Application
Version=1.0
Name={APP_NAME}
Comment={APP_NAME} startup script
Exec={exe}
Icon={FLATPAK_APP_ID}
StartupWMClass={FLATPAK_APP_ID}
StartupNotify=false
Terminal=false
"
    ))
}

fn flatpak_desktop_contents() -> String {
    format!(
        "\
[Desktop Entry]
Type=Application
Name={APP_NAME}
Icon={FLATPAK_APP_ID}
StartupWMClass={FLATPAK_APP_ID}
X-XDP-Autostart={FLATPAK_APP_ID}
Exec=flatpak run --command={APP_NAME} {FLATPAK_APP_ID}
X-Flatpak={FLATPAK_APP_ID}
"
    )
}

fn remove_legacy_entries(dir: &PathBuf) {
    let _ = fs::remove_file(dir.join(LEGACY_DESKTOP));
    if !is_flatpak() {
        let _ = fs::remove_file(dir.join(format!("{FLATPAK_APP_ID}.desktop")));
    }
}

fn write_desktop_file(enabled: bool) -> Result<(), String> {
    let dir = autostart_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    remove_legacy_entries(&dir);
    let path = desktop_path()?;
    if enabled {
        let contents = if is_flatpak() {
            flatpak_desktop_contents()
        } else {
            native_desktop_contents()?
        };
        fs::write(&path, contents).map_err(|e| e.to_string())?;
    } else {
        let _ = fs::remove_file(&path);
    }
    Ok(())
}

fn desktop_file_enabled() -> Result<bool, String> {
    let path = desktop_path()?;
    if !path.is_file() {
        return Ok(false);
    }
    if !is_flatpak() {
        return Ok(true);
    }
    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(contents.contains("flatpak run") && contents.contains(FLATPAK_APP_ID))
}

fn marker_enabled() -> bool {
    marker_path().is_some_and(|p| p.is_file())
}

fn set_marker(enabled: bool) {
    let Some(path) = marker_path() else {
        return;
    };
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, b"1\n");
    } else {
        let _ = fs::remove_file(&path);
    }
}

#[cfg(target_os = "linux")]
fn set_via_background_portal(enabled: bool) -> Result<(), String> {
    use ashpd::desktop::background::Background;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    rt.block_on(async move {
        let request = Background::request()
            .reason("Launch emobie when you sign in")
            .auto_start(enabled)
            .command(["emobie"])
            .dbus_activatable(false);
        let response = request
            .send()
            .await
            .map_err(|e| format!("Background portal: {e}"))?
            .response()
            .map_err(|e| format!("Background portal denied: {e}"))?;
        if enabled && !response.auto_start() {
            return Err(
                "Autostart was not granted by the desktop portal. Check Settings → Apps."
                    .into(),
            );
        }
        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn set_via_background_portal(_enabled: bool) -> Result<(), String> {
    Err("Background portal is Linux-only".into())
}

#[tauri::command]
pub fn is_launch_on_startup() -> Result<bool, String> {
    if is_flatpak() {
        if marker_enabled() {
            return Ok(true);
        }
        // GitHub Flatpak may still have a visible autostart desktop file.
        if desktop_file_enabled().unwrap_or(false) {
            return Ok(true);
        }
        return Ok(false);
    }
    desktop_file_enabled()
}

#[tauri::command]
pub fn set_launch_on_startup(enabled: bool) -> Result<(), String> {
    if is_flatpak() {
        match set_via_background_portal(enabled) {
            Ok(()) => {
                set_marker(enabled);
                // Best-effort desktop file when the sandbox can write autostart.
                let _ = write_desktop_file(enabled);
                return Ok(());
            }
            Err(portal_err) => {
                // GitHub Flatpak finish-args allow xdg-config/autostart:create.
                match write_desktop_file(enabled) {
                    Ok(()) => {
                        set_marker(enabled);
                        return Ok(());
                    }
                    Err(_) => return Err(portal_err),
                }
            }
        }
    }
    write_desktop_file(enabled)
}
