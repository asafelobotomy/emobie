//! Download a GitHub release asset and install it for this package type.

use serde::Serialize;
use std::fs::{self, File};
use std::io::{copy, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::native::install_native_from_deb;

const USER_AGENT: &str = concat!("emobie/", env!("CARGO_PKG_VERSION"));
const ALLOWED_PREFIX: &str =
    "https://github.com/asafelobotomy/emobie/releases/download/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallKind {
    Flatpak,
    AppImage,
    Deb,
    Rpm,
    Native,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyUpdateResult {
    pub ok: bool,
    pub detail: String,
    /// True when the user should quit and relaunch emobie.
    pub restart_required: bool,
}

pub fn detect_install_kind() -> InstallKind {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return InstallKind::Flatpak;
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return InstallKind::AppImage;
    }
    if let Ok(exe) = std::env::current_exe() {
        if exe
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("AppImage"))
        {
            return InstallKind::AppImage;
        }
        let path = exe.to_string_lossy();
        if path.starts_with("/usr/") {
            if Path::new("/var/lib/dpkg/info/emobie.list").exists()
                || Path::new("/var/lib/dpkg/info/emobie.md5sums").exists()
            {
                return InstallKind::Deb;
            }
            if Command::new("rpm")
                .args(["-q", "emobie"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return InstallKind::Rpm;
            }
            // Packaged under /usr but unknown manager — prefer deb if apt exists.
            if which("apt-get") || which("dpkg") {
                return InstallKind::Deb;
            }
            if which("dnf") || which("zypper") || which("rpm") {
                return InstallKind::Rpm;
            }
        }
    }
    InstallKind::Native
}

fn which(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn cache_dir() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache"))
        })
        .ok_or_else(|| "HOME is not set".to_string())?;
    let dir = base.join("emobie").join("updates");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn validate_download_url(url: &str) -> Result<(), String> {
    if !url.starts_with(ALLOWED_PREFIX) {
        return Err("Refusing download from unexpected host/path.".into());
    }
    Ok(())
}

fn download_asset(url: &str, dest: &Path) -> Result<(), String> {
    validate_download_url(url)?;
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/octet-stream")
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let mut reader = response.into_reader();
    let mut file = File::create(dest).map_err(|e| e.to_string())?;
    copy(&mut reader, &mut file).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn run_checked(cmd: &mut Command) -> Result<(), String> {
    let output = cmd.output().map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or("command failed");
    Err(detail.to_string())
}

fn install_flatpak(path: &Path) -> Result<(), String> {
    let path_str = path.display().to_string();
    // From inside the sandbox, talk to the host Flatpak.
    if std::env::var_os("FLATPAK_ID").is_some() && which("flatpak-spawn") {
        return run_checked(Command::new("flatpak-spawn").args([
            "--host",
            "flatpak",
            "install",
            "--user",
            "-y",
            "--noninteractive",
            &path_str,
        ]));
    }
    run_checked(Command::new("flatpak").args([
        "install",
        "--user",
        "-y",
        "--noninteractive",
        &path_str,
    ]))
}

fn install_appimage(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    }

    if let Ok(current) = std::env::var("APPIMAGE") {
        let current_path = PathBuf::from(&current);
        let backup = current_path.with_extension("AppImage.bak");
        let _ = fs::remove_file(&backup);
        fs::rename(&current_path, &backup).map_err(|e| {
            format!("Could not backup current AppImage ({e})")
        })?;
        if let Err(err) = fs::rename(path, &current_path) {
            let _ = fs::rename(&backup, &current_path);
            return Err(format!("Could not replace AppImage ({err})"));
        }
        let _ = fs::remove_file(&backup);
        return Ok(());
    }

    // Native / portable: install beside ~/.local/bin
    let dest = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".local/bin/emobie.AppImage"))
        .ok_or_else(|| "HOME is not set".to_string())?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::rename(path, &dest).or_else(|_| {
        fs::copy(path, &dest).map(|_| ()).map_err(|e| e.to_string())
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms).map_err(|e| e.to_string())?;
    }
    // Keep a stable launcher with WebKit workarounds (blank UI on some Wayland sessions).
    let launcher = dest
        .parent()
        .map(|p| p.join("emobie"))
        .ok_or_else(|| "invalid install path".to_string())?;
    let script = format!(
        "#!/bin/sh\nexport WEBKIT_DISABLE_DMABUF_RENDERER=1\nexport WEBKIT_DISABLE_COMPOSITING_MODE=1\nexec \"{}\" \"$@\"\n",
        dest.display()
    );
    fs::write(&launcher, script).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&launcher)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&launcher, perms).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn install_deb(path: &Path) -> Result<(), String> {
    if which("apt-get") {
        return run_checked(Command::new("pkexec").args([
            "env",
            "DEBIAN_FRONTEND=noninteractive",
            "apt-get",
            "install",
            "-y",
            &path.display().to_string(),
        ]));
    }
    run_checked(
        Command::new("pkexec")
            .arg("dpkg")
            .arg("-i")
            .arg(path),
    )
}

fn install_rpm(path: &Path) -> Result<(), String> {
    if which("dnf") {
        return run_checked(
            Command::new("pkexec")
                .args(["dnf", "install", "-y"])
                .arg(path),
        );
    }
    if which("zypper") {
        return run_checked(
            Command::new("pkexec")
                .args(["zypper", "--non-interactive", "install", "--allow-unsigned-rpm"])
                .arg(path),
        );
    }
    run_checked(
        Command::new("pkexec")
            .args(["rpm", "-Uvh"])
            .arg(path),
    )
}

pub fn apply_update(download_url: String, asset_name: String) -> Result<ApplyUpdateResult, String> {
    validate_download_url(&download_url)?;
    if asset_name.contains('/') || asset_name.contains("..") {
        return Err("Invalid asset name.".into());
    }

    let kind = detect_install_kind();
    let expected = match kind {
        InstallKind::Flatpak => ".flatpak",
        InstallKind::AppImage => ".AppImage",
        InstallKind::Native | InstallKind::Deb => ".deb",
        InstallKind::Rpm => ".rpm",
    };
    if !asset_name.ends_with(expected) {
        return Err(format!(
            "Asset {asset_name} does not match this install ({expected})."
        ));
    }

    let dir = cache_dir()?;
    let dest = dir.join(&asset_name);
    let _ = fs::remove_file(&dest);
    download_asset(&download_url, &dest)?;

    let result = match kind {
        InstallKind::Flatpak => install_flatpak(&dest).map(|_| ApplyUpdateResult {
            ok: true,
            detail: "Flatpak updated. Quit and relaunch emobie to finish.".into(),
            restart_required: true,
        }),
        InstallKind::AppImage => install_appimage(&dest).map(|_| ApplyUpdateResult {
            ok: true,
            detail: "AppImage replaced. Quit and relaunch emobie to finish.".into(),
            restart_required: true,
        }),
        InstallKind::Native => install_native_from_deb(&dest).map(|_| ApplyUpdateResult {
            ok: true,
            detail: "Installed ~/.local/bin/emobie-bin. Quit and relaunch emobie to finish."
                .into(),
            restart_required: true,
        }),
        InstallKind::Deb => install_deb(&dest).map(|_| ApplyUpdateResult {
            ok: true,
            detail: "Package installed. Quit and relaunch emobie to finish.".into(),
            restart_required: true,
        }),
        InstallKind::Rpm => install_rpm(&dest).map(|_| ApplyUpdateResult {
            ok: true,
            detail: "Package installed. Quit and relaunch emobie to finish.".into(),
            restart_required: true,
        }),
    };

    let _ = fs::remove_file(&dest);
    result
}
