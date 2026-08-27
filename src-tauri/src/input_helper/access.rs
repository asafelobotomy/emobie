//! Keyboard-access setup via pkexec (host path under Flatpak).

use super::unix;
use super::InputHelperStatus;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const SYSTEM_SETUP: &str = "/usr/share/emobie/setup-input-access.sh";

fn in_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

pub fn host_setup_hint() -> String {
    if in_flatpak() {
        "Flatpak cannot grant keyboard access inside the sandbox. On the host \
run: bash packaging/install-inputd-user.sh && \
pkexec bash ~/.local/share/emobie/setup-input-access.sh \
— then retry Expand."
            .into()
    } else {
        "Run: pkexec bash ~/.local/share/emobie/setup-input-access.sh \
(or packaging/setup-input-access.sh), then retry."
            .into()
    }
}

fn sandbox_setup_scripts() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(SYSTEM_SETUP)];
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".local/share/emobie/setup-input-access.sh"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("setup-input-access.sh"));
            paths.push(dir.join("../../../packaging/setup-input-access.sh"));
        }
    }
    paths
}

fn host_setup_candidates() -> Vec<String> {
    let mut paths = vec![SYSTEM_SETUP.to_string()];
    if let Ok(home) = std::env::var("HOME") {
        paths.push(format!("{home}/.local/share/emobie/setup-input-access.sh"));
    }
    paths
}

fn host_file_exists(path: &str) -> bool {
    Command::new("flatpak-spawn")
        .args(["--host", "test", "-f", path])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

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

/// Prefer direct pkexec of the annotated system script; bash-wrap user copies.
fn pkexec_args(script: &str) -> Vec<String> {
    if script == SYSTEM_SETUP {
        vec![script.to_string()]
    } else {
        vec!["/usr/bin/bash".into(), script.to_string()]
    }
}

fn run_pkexec(script: &str, flatpak: bool) -> Result<(), String> {
    let args = pkexec_args(script);
    let mut cmd = if flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pkexec").args(&args);
        c
    } else {
        let mut c = Command::new("pkexec");
        c.args(&args);
        c
    };
    with_session_env(&mut cmd);

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

fn resolve_setup_script() -> Result<(String, bool), String> {
    let flatpak = in_flatpak();
    if flatpak {
        for path in host_setup_candidates() {
            if host_file_exists(&path) {
                return Ok((path, true));
            }
        }
        for path in sandbox_setup_scripts() {
            if path.is_file() {
                return Ok((path.to_string_lossy().into_owned(), true));
            }
        }
        return Err(host_setup_hint());
    }

    let script = sandbox_setup_scripts()
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "setup-input-access.sh not found — install the emobie package or run \
packaging/setup-input-access.sh"
                .to_string()
        })?;
    Ok((script.to_string_lossy().into_owned(), false))
}

pub fn with_flatpak_flag(mut status: InputHelperStatus) -> InputHelperStatus {
    status.flatpak = in_flatpak();
    if !status.daemon
        && status.flatpak
        && !status.detail.contains("Flatpak needs a host helper")
    {
        status.detail = format!(
            "{} Flatpak needs a host helper — run packaging/install-inputd-user.sh on the host.",
            status.detail
        );
    }
    status
}

/// Polkit setup + restart helper; returns fresh status (can_listen after ACLs).
pub fn run_access_setup() -> Result<InputHelperStatus, String> {
    let (script, flatpak) = resolve_setup_script()?;
    run_pkexec(&script, flatpak)?;
    let mut status = with_flatpak_flag(unix::restart_helper());
    if status.can_listen {
        status.detail =
            "Keyboard access ready — Expand as you type can watch keys now.".into();
    } else if status.daemon {
        status.detail = "Helper restarted but keyboard devices are still closed. \
If session ACLs failed, log out/in once so the emobie-input group applies."
            .into();
    } else {
        status.detail = format!(
            "Access script finished but helper is not running. {}",
            status.detail
        );
    }
    Ok(status)
}
