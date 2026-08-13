//! Launch-on-startup registration that works for both native and Flatpak installs.
//!
//! The stock `tauri-plugin-autostart` writes `Exec=/app/bin/...`, which the host
//! session manager cannot run. Flatpak entries must use `flatpak run`.

use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "emobie";
const FLATPAK_APP_ID: &str = "io.github.asafelobotomy.emobie";
const LEGACY_DESKTOP: &str = "emobie.desktop";

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

fn flatpak_desktop_contents() -> String {
    format!(
        "\
[Desktop Entry]
Type=Application
Name={APP_NAME}
X-XDP-Autostart={FLATPAK_APP_ID}
Exec=flatpak run --command={APP_NAME} {FLATPAK_APP_ID}
X-Flatpak={FLATPAK_APP_ID}
"
    )
}

fn native_desktop_contents() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let exe = exe
        .canonicalize()
        .unwrap_or(exe)
        .display()
        .to_string()
        .replace(' ', "\\ ");
    Ok(format!(
        "\
[Desktop Entry]
Type=Application
Version=1.0
Name={APP_NAME}
Comment={APP_NAME} startup script
Exec={exe}
StartupNotify=false
Terminal=false
"
    ))
}

fn remove_legacy_entries(dir: &PathBuf) {
    // Broken plugin entry used sandbox-only Exec=/app/bin/emobie
    let _ = fs::remove_file(dir.join(LEGACY_DESKTOP));
    if !is_flatpak() {
        let _ = fs::remove_file(dir.join(format!("{FLATPAK_APP_ID}.desktop")));
    }
}

#[tauri::command]
pub fn is_launch_on_startup() -> Result<bool, String> {
    let path = desktop_path()?;
    if !path.is_file() {
        return Ok(false);
    }
    if !is_flatpak() {
        return Ok(true);
    }
    let contents = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    Ok(contents.contains("flatpak run") && contents.contains(FLATPAK_APP_ID))
}

#[tauri::command]
pub fn set_launch_on_startup(enabled: bool) -> Result<(), String> {
    let dir = autostart_dir()?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    remove_legacy_entries(&dir);

    let path = desktop_path()?;
    if enabled {
        let contents = if is_flatpak() {
            flatpak_desktop_contents()
        } else {
            native_desktop_contents()?
        };
        fs::write(&path, contents).map_err(|err| err.to_string())?;
    } else {
        let _ = fs::remove_file(&path);
    }
    Ok(())
}
