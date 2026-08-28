//! Install bundled emobie-inputd on the host (AppImage / Flatpak).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
    dirs.push(PathBuf::from("/app/share/emobie"));
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

fn host_helper_installed() -> bool {
    let Some(home) = host_home() else {
        return false;
    };
    home.join(".local/bin/emobie-inputd").is_file()
}

fn run_host_script(script: &str) -> bool {
    if std::env::var_os("FLATPAK_ID").is_some() {
        Command::new("flatpak-spawn")
            .args(["--host", "bash", "-s"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .and_then(|mut child| {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(script.as_bytes())?;
                }
                child.wait()
            })
            .map(|status| status.success())
            .unwrap_or(false)
    } else {
        Command::new("bash")
            .arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
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

    if std::env::var_os("FLATPAK_ID").is_some() {
        Command::new("flatpak-spawn")
            .args([
                "--host",
                "sh",
                "-c",
                "mkdir -p \"$HOME/.local/share/emobie\" && \
                 tar xzf - -C \"$HOME/.local/share/emobie\" && \
                 bash \"$HOME/.local/share/emobie/bootstrap-inputd-host.sh\" \
                   \"$HOME/.local/share/emobie/emobie-inputd\"",
            ])
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
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        let data = format!("{home}/.local/share/emobie");
        let script = format!(
            "set -euo pipefail; \
             mkdir -p '{data}'; \
             tar xzf - -C '{data}'; \
             bash '{data}/bootstrap-inputd-host.sh' '{data}/emobie-inputd'"
        );
        Command::new("bash")
            .arg("-c")
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
    }
}

fn install_from_loose_dir(dir: &Path) -> bool {
    let binary = dir.join("emobie-inputd");
    let bootstrap = dir.join("bootstrap-inputd-host.sh");
    if !binary.is_file() || !bootstrap.is_file() {
        return false;
    }
    let script = format!(
        "exec bash '{}' '{}'",
        bootstrap.display(),
        binary.display()
    );
    run_host_script(&script)
}

/// Install host helper from AppImage/Flatpak bundle when missing.
pub fn try_bootstrap_host_helper() -> bool {
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
