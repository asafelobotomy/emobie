//! Install bundled emobie-inputd on the host (AppImage / Flatpak).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

/// Members allowed in the host bootstrap tarball (must match scripts/stage-inputd.sh).
const TAR_MEMBERS: &[&str] = &[
    "emobie-inputd",
    "bootstrap-inputd-host.sh",
    "setup-input-access.sh",
    "99-emobie-input.rules",
    "io.github.asafelobotomy.emobie.inputd.policy",
    "selinux/emobie-inputd.te",
];

fn bundled_tarball_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(appdir) = std::env::var("APPDIR") {
        paths.push(
            PathBuf::from(appdir)
                .join("usr/share/emobie/inputd-host-bundle.tgz"),
        );
    }
    paths.push(PathBuf::from(
        "/app/share/emobie/inputd-host-bundle.tgz",
    ));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("usr/share/emobie/inputd-host-bundle.tgz"));
        }
    }
    paths
}

fn bundled_loose_paths() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(appdir) = std::env::var("APPDIR") {
        dirs.push(PathBuf::from(appdir).join("usr/share/emobie"));
    }
    // /app is only visible inside Flatpak — host bash cannot read it.
    if std::env::var_os("FLATPAK_ID").is_none() {
        dirs.push(PathBuf::from("/app/share/emobie"));
    }
    dirs
}

fn host_home() -> Option<PathBuf> {
    if std::env::var_os("FLATPAK_ID").is_some() {
        let output = Command::new("flatpak-spawn")
            .args(["--host", "printenv", "HOME"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if home.is_empty() {
            None
        } else {
            Some(PathBuf::from(home))
        }
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn host_data_dir() -> Option<PathBuf> {
    host_home().map(|home| home.join(".local/share/emobie"))
}

fn host_helper_path() -> Option<PathBuf> {
    host_home().map(|home| home.join(".local/bin/emobie-inputd"))
}

fn path_safe_for_shell(path: &Path) -> bool {
    let s = path.to_string_lossy();
    !s.is_empty()
        && !s.contains('\0')
        && !s.contains('\'')
        && !s.contains('"')
        && !s.contains('$')
        && !s.contains('`')
        && !s.contains(';')
        && !s.contains('|')
        && !s.contains('&')
}

fn host_file_executable(path: &Path) -> bool {
    if std::env::var_os("FLATPAK_ID").is_some() {
        let path_str = path.to_string_lossy();
        return Command::new("flatpak-spawn")
            .args(["--host", "test", "-x", path_str.as_ref()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    path.is_file()
}

fn host_helper_installed() -> bool {
    host_helper_path()
        .map(|p| host_file_executable(&p))
        .unwrap_or(false)
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn host_mtime(path: &Path) -> Option<SystemTime> {
    if std::env::var_os("FLATPAK_ID").is_none() {
        return mtime(path);
    }
    let path_str = path.to_string_lossy();
    let output = Command::new("flatpak-spawn")
        .args([
            "--host",
            "stat",
            "-c",
            "%Y",
            path_str.as_ref(),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let secs: u64 = String::from_utf8_lossy(&output.stdout).trim().parse().ok()?;
    Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
}

/// Parse `x.y.z` (optional leading text) into comparable triple.
fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let token = raw
        .split_whitespace()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

fn host_run_version(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    let output = if std::env::var_os("FLATPAK_ID").is_some() {
        Command::new("flatpak-spawn")
            .args(["--host", path_str.as_ref(), "--version"])
            .output()
            .ok()?
    } else {
        Command::new(path).arg("--version").output().ok()?
    };
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(|l| l.trim().to_string())
}

fn version_lt(installed: &str, bundled: &str) -> Option<bool> {
    let a = parse_semver(installed)?;
    let b = parse_semver(bundled)?;
    Some(a < b)
}

/// Bundled helper version matches the app crate when staged together.
fn bundled_helper_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// True when the bundled helper should replace the installed one.
/// Prefer semver (installed < bundled); fall back to mtime when versions unknown.
fn bundled_helper_newer_than_installed(tarball: &Path) -> bool {
    let Some(installed_path) = host_helper_path() else {
        return true;
    };
    if !host_file_executable(&installed_path) {
        return true;
    }
    if let Some(installed_ver) = host_run_version(&installed_path) {
        if let Some(older) = version_lt(&installed_ver, bundled_helper_version()) {
            // Refuse to downgrade or no-op replace when installed >= bundled.
            return older;
        }
    }
    let Some(bundle_mtime) = mtime(tarball) else {
        return false;
    };
    match host_mtime(&installed_path) {
        Some(installed_mtime) => bundle_mtime > installed_mtime,
        None => true,
    }
}

fn run_host_bootstrap(bootstrap: &Path, binary: &Path) -> bool {
    let mut cmd = if std::env::var_os("FLATPAK_ID").is_some() {
        let mut c = Command::new("flatpak-spawn");
        c.args(["--host", "bash"]);
        c.arg(bootstrap).arg(binary);
        c
    } else {
        let mut c = Command::new("bash");
        c.arg(bootstrap).arg(binary);
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn extract_tarball_to(data: &Path, bytes: &[u8]) -> bool {
    if fs_create_dir_all(data).is_err() {
        return false;
    }
    let mut cmd = Command::new("tar");
    cmd.args(["xzf", "-", "-C"])
        .arg(data)
        .args(["--no-absolute-names", "--no-overwrite-dir"])
        .args(TAR_MEMBERS)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(bytes)?;
            }
            child.wait()
        })
        .map(|status| status.success())
        .unwrap_or(false)
}

fn fs_create_dir_all(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

fn install_from_tarball(tarball: &Path) -> bool {
    let mut file = match std::fs::File::open(tarball) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return false;
    }

    let Some(data) = host_data_dir() else {
        return false;
    };
    if !path_safe_for_shell(&data) {
        return false;
    }

    if std::env::var_os("FLATPAK_ID").is_some() {
        // Host-side extract via flatpak-spawn; paths come from host HOME, not the sandbox.
        let data_str = data.to_string_lossy();
        let members = TAR_MEMBERS
            .iter()
            .map(|m| format!("'{m}'"))
            .collect::<Vec<_>>()
            .join(" ");
        let script = format!(
            "set -euo pipefail; \
             mkdir -p '{data_str}'; \
             tar xzf - -C '{data_str}' --no-absolute-names --no-overwrite-dir {members}; \
             exec bash '{data_str}/bootstrap-inputd-host.sh' '{data_str}/emobie-inputd'"
        );
        Command::new("flatpak-spawn")
            .args(["--host", "bash", "-c"])
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .and_then(|mut child| {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(&bytes)?;
                }
                child.wait()
            })
            .map(|status| status.success())
            .unwrap_or(false)
    } else if !extract_tarball_to(&data, &bytes) {
        false
    } else {
        run_host_bootstrap(
            &data.join("bootstrap-inputd-host.sh"),
            &data.join("emobie-inputd"),
        )
    }
}

fn install_from_loose_dir(dir: &Path) -> bool {
    // Flatpak host cannot read /app paths — only use host-extracted tarball flow.
    if std::env::var_os("FLATPAK_ID").is_some() {
        return false;
    }
    let binary = dir.join("emobie-inputd");
    let bootstrap = dir.join("bootstrap-inputd-host.sh");
    if !binary.is_file() || !bootstrap.is_file() {
        return false;
    }
    // Same gate as tarball: do not clobber a newer host helper with an older bundle.
    if let (Some(installed), Some(bundled_ver)) = (
        host_helper_path().filter(|p| host_file_executable(p)),
        host_run_version(&binary),
    ) {
        if let Some(installed_ver) = host_run_version(&installed) {
            if version_lt(&installed_ver, &bundled_ver) == Some(false) {
                return true; // already >= bundled
            }
        }
    }
    run_host_bootstrap(&bootstrap, &binary)
}

/// Install or refresh host helper from AppImage/Flatpak bundle when missing or stale.
pub fn try_bootstrap_host_helper() -> bool {
    // Refresh when a bundled tarball is newer than the installed helper.
    for path in bundled_tarball_paths() {
        if path.is_file() && bundled_helper_newer_than_installed(&path) {
            if install_from_tarball(&path) {
                return host_helper_installed();
            }
        }
    }
    if host_helper_installed() {
        return true;
    }
    for path in bundled_tarball_paths() {
        if path.is_file() && install_from_tarball(&path) {
            return host_helper_installed();
        }
    }
    for dir in bundled_loose_paths() {
        if dir.is_dir() && install_from_loose_dir(&dir) {
            return host_helper_installed();
        }
    }
    false
}

/// Force re-bootstrap from the newest bundled tarball (used after app updates).
pub fn refresh_host_helper() -> bool {
    for path in bundled_tarball_paths() {
        if path.is_file() && install_from_tarball(&path) {
            return host_helper_installed();
        }
    }
    for dir in bundled_loose_paths() {
        if dir.is_dir() && install_from_loose_dir(&dir) {
            return host_helper_installed();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{parse_semver, version_lt};

    #[test]
    fn parses_version_line() {
        assert_eq!(parse_semver("emobie-inputd 0.6.14"), Some((0, 6, 14)));
        assert_eq!(parse_semver("0.6.14"), Some((0, 6, 14)));
    }

    #[test]
    fn refuses_downgrade_when_installed_newer_or_equal() {
        assert_eq!(version_lt("emobie-inputd 0.6.14", "0.6.14"), Some(false));
        assert_eq!(version_lt("0.7.0", "0.6.14"), Some(false));
        assert_eq!(version_lt("0.6.13", "0.6.14"), Some(true));
    }
}
