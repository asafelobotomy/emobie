//! Resolve, stage, and run the Polkit-annotated setup script.
//!
//! Trust model: the only things ever executed or installed as root are
//! (a) the package-managed `/usr/share/emobie/setup-input-access.sh`, or
//! (b) the bytes embedded in this binary (see `assets`), staged into
//! `/usr/local/share/emobie/`. No user-writable file is ever copied to root.

use super::assets::STAGED_FILES;
use super::permanent::{host_setup_hint, in_flatpak, LOCAL_SETUP, SYSTEM_SETUP};
use std::io::Write;
use std::process::{Command, Stdio};

const LOCAL_DIR: &str = "/usr/local/share/emobie";

fn with_session_env(cmd: &mut Command) {
    for key in [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XAUTHORITY",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_RUNTIME_DIR",
    ] {
        if let Ok(value) = std::env::var(key) {
            cmd.env(key, value);
        }
    }
}

/// Root-owned file contents, or `None` when missing/unreadable. Only used to
/// decide whether a staged copy is already current — the data never gets
/// executed by this process.
fn read_installed(path: &str, flatpak: bool) -> Option<Vec<u8>> {
    let output = if flatpak {
        Command::new("flatpak-spawn")
            .args(["--host", "cat", path])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?
    } else {
        return std::fs::read(path).ok();
    };
    output.status.success().then_some(output.stdout)
}

/// `pkexec install -D -m MODE /dev/stdin DEST`, feeding `bytes` on stdin so the
/// data root installs is exactly what we hold in memory (no path to swap).
fn pkexec_install_bytes(flatpak: bool, mode: &str, bytes: &[u8], dest: &str) -> Result<(), String> {
    let install_args = ["install", "-D", "-m", mode, "/dev/stdin", dest];
    let mut cmd = if flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pkexec").args(install_args);
        c
    } else {
        let mut c = Command::new("pkexec");
        c.args(install_args);
        c
    };
    with_session_env(&mut cmd);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not stage {dest} ({e}). {}", host_setup_hint()))?;
    if let Some(mut stdin) = child.stdin.take() {
        // Errors surface through the exit status below (e.g. auth cancelled).
        let _ = stdin.write_all(bytes);
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Could not stage {dest} ({e}). {}", host_setup_hint()))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if detail.is_empty() {
        format!("Could not install {dest}. {}", host_setup_hint())
    } else {
        format!("Keyboard access setup staging ({dest}): {detail}")
    })
}

/// Stage every embedded asset that is missing or differs from the installed
/// copy into `/usr/local/share/emobie/` (root-owned).
fn stage_embedded_assets(flatpak: bool) -> Result<(), String> {
    for (bytes, mode, rel) in STAGED_FILES {
        let dest = format!("{LOCAL_DIR}/{rel}");
        if read_installed(&dest, flatpak).as_deref() == Some(bytes) {
            continue;
        }
        pkexec_install_bytes(flatpak, mode, bytes, &dest)?;
    }
    Ok(())
}

pub(super) fn ensure_polkit_annotated_setup(script: &str, flatpak: bool) -> Result<String, String> {
    if script == SYSTEM_SETUP {
        // Package-managed and root-owned: trusted as-is.
        return Ok(script.to_string());
    }
    stage_embedded_assets(flatpak)?;
    Ok(LOCAL_SETUP.to_string())
}

pub(super) fn run_pkexec(script: &str, flatpak: bool) -> Result<(), String> {
    let mut cmd = if flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pkexec").arg(script);
        c
    } else {
        let mut c = Command::new("pkexec");
        c.arg(script);
        c
    };
    with_session_env(&mut cmd);
    // pkexec scrubs the environment; the script resolves the invoking user
    // from PKEXEC_UID.
    let output = cmd.stdin(Stdio::null()).output().map_err(|e| {
        if flatpak {
            format!("flatpak-spawn --host failed ({e}). {}", host_setup_hint())
        } else {
            format!("Could not launch pkexec ({e}). {}", host_setup_hint())
        }
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or("cancelled or failed");
    Err(format!("Keyboard access setup: {detail}"))
}

/// Which script to run as root: the package's, if installed, otherwise our
/// staged embedded copy (staged by `ensure_polkit_annotated_setup`).
pub(super) fn resolve_setup_script() -> Result<(String, bool), String> {
    let flatpak = in_flatpak();
    let system_present = if flatpak {
        super::permanent::host_file_exists(SYSTEM_SETUP)
    } else {
        std::path::Path::new(SYSTEM_SETUP).is_file()
    };
    let script = if system_present { SYSTEM_SETUP } else { LOCAL_SETUP };
    Ok((script.to_string(), flatpak))
}

#[cfg(test)]
mod tests {
    use super::super::assets::{POLKIT_POLICY, SELINUX_TE, SETUP_SCRIPT, STAGED_FILES, UDEV_RULES};

    #[test]
    fn embedded_assets_are_present_and_sane() {
        assert!(SETUP_SCRIPT.starts_with(b"#!/usr/bin/env bash"));
        assert!(UDEV_RULES.windows(6).any(|w| w == b"uinput"));
        assert!(POLKIT_POLICY.starts_with(b"<?xml"));
        assert!(!SELINUX_TE.is_empty());
        assert_eq!(STAGED_FILES.len(), 4);
    }

    #[test]
    fn shipped_udev_rule_grants_no_keyboard_read() {
        let text = String::from_utf8_lossy(UDEV_RULES);
        let active: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
            .collect();
        assert!(active.iter().all(|l| !l.contains("event*")), "{active:?}");
    }
}
