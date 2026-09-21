//! Download a GitHub release asset and install it for this package type.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::native::install_native_from_deb;
use super::verified_install::{install_deb, install_rpm};

const USER_AGENT: &str = concat!("emobie/", env!("CARGO_PKG_VERSION"));
pub const ALLOWED_PREFIX: &str =
    "https://github.com/asafelobotomy/emobie/releases/download/";
/// Packages are tens of MB; refuse anything absurd rather than fill the disk.
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

/// Best-effort: refresh host inputd after an app/package update.
fn refresh_inputd_after_update(kind: InstallKind) {
    #[cfg(target_os = "linux")]
    {
        match kind {
            InstallKind::AppImage | InstallKind::Flatpak => {
                let _ = crate::input_helper::bootstrap::refresh_host_helper();
                let _ = crate::input_helper::unix::try_restart_inputd_unit();
            }
            InstallKind::Native => {
                let _ = crate::input_helper::unix::try_restart_inputd_unit();
            }
            InstallKind::Deb | InstallKind::Rpm => {
                if let Ok(home) = std::env::var("HOME") {
                    let user_unit = PathBuf::from(&home)
                        .join(".config/systemd/user/emobie-inputd.service");
                    if Path::new("/usr/bin/emobie-inputd").is_file() && user_unit.is_file() {
                        let _ = fs::remove_file(&user_unit);
                    }
                }
                let _ = crate::input_helper::unix::try_restart_inputd_unit();
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = kind;
    }
}

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

pub(super) fn which(bin: &str) -> bool {
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
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&dir).map_err(|e| e.to_string())?;
    // Downloads are later installed as root — keep other users out of the dir
    // even if an older version created it with looser permissions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

fn validate_download_url(url: &str) -> Result<(), String> {
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("Invalid download URL.".into());
    }
    if !url.starts_with(ALLOWED_PREFIX) {
        return Err("Refusing download from unexpected host/path.".into());
    }
    Ok(())
}

pub(super) fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Download `url` to `dest` (created exclusively, mode 0600) and verify its
/// SHA-256 against `expected_sha256`. On any failure the partial file is removed.
fn download_asset(url: &str, dest: &Path, expected_sha256: &str) -> Result<(), String> {
    validate_download_url(url)?;
    let result = (|| {
        let response = ureq::get(url)
            .set("User-Agent", USER_AGENT)
            .set("Accept", "application/octet-stream")
            .call()
            .map_err(|e| format!("Download failed: {e}"))?;
        let mut reader = response.into_reader().take(MAX_DOWNLOAD_BYTES + 1);
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut file = opts.open(dest).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        let mut total = 0u64;
        loop {
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > MAX_DOWNLOAD_BYTES {
                return Err("Download is larger than expected.".to_string());
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
        file.flush().map_err(|e| e.to_string())?;
        if lower_hex(&hasher.finalize()) != expected_sha256.to_ascii_lowercase() {
            return Err(
                "Downloaded file failed checksum verification — not installing.".to_string(),
            );
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(dest);
    }
    result
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
            if fs::copy(path, &current_path).is_err() {
                let _ = fs::rename(&backup, &current_path);
                return Err(format!("Could not replace AppImage ({err})"));
            }
            let _ = fs::remove_file(path);
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
        "#!/bin/sh\n\
         # WebKitGTK often blanks or crashes (EGL_BAD_PARAMETER) under Plasma Wayland.\n\
         export WEBKIT_DISABLE_DMABUF_RENDERER=1\n\
         export WEBKIT_DISABLE_COMPOSITING_MODE=1\n\
         # Prefer XWayland for the WebView when Wayland EGL is broken.\n\
         if [ \"${{XDG_SESSION_TYPE:-}}\" = wayland ] || [ -n \"${{WAYLAND_DISPLAY:-}}\" ]; then\n\
           export GDK_BACKEND=\"${{EMOBIE_GDK_BACKEND:-x11}}\"\n\
         fi\n\
         exec \"{}\" \"$@\"\n",
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

pub fn apply_update(
    release_tag: String,
    download_url: String,
    asset_name: String,
) -> Result<ApplyUpdateResult, String> {
    validate_download_url(&download_url)?;
    if asset_name.contains('/')
        || asset_name.contains("..")
        || asset_name.starts_with('-')
        || asset_name.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("Invalid asset name.".into());
    }

    let kind = detect_install_kind();
    let expected_sha256 =
        super::verify_update_asset(&release_tag, &download_url, &asset_name, kind)?;
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
    download_asset(&download_url, &dest, &expected_sha256)?;

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
        InstallKind::Deb => install_deb(&dest, &expected_sha256).map(|_| ApplyUpdateResult {
            ok: true,
            detail: "Package installed. Quit and relaunch emobie to finish.".into(),
            restart_required: true,
        }),
        InstallKind::Rpm => install_rpm(&dest, &expected_sha256).map(|_| ApplyUpdateResult {
            ok: true,
            detail: "Package installed. Quit and relaunch emobie to finish.".into(),
            restart_required: true,
        }),
    };

    if result.is_ok() {
        refresh_inputd_after_update(kind);
    }

    let _ = fs::remove_file(&dest);
    result
}
