//! Start/stop emobie-inputd via systemd or a detached binary.

use super::socket::request;
use super::{offline_status, status_from_resp};
use crate::input_helper::InputHelperStatus;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Poll until the daemon socket accepts Status (does not require can_inject).
fn wait_until_running(attempts: u32) -> Option<InputHelperStatus> {
    for _ in 0..attempts {
        thread::sleep(Duration::from_millis(150));
        if let Ok(resp) = request(serde_json::json!({ "cmd": "status" })) {
            return Some(status_from_resp(resp));
        }
    }
    None
}

fn is_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

fn run_host_program(program: &str, args: &[&str]) -> bool {
    let status = if is_flatpak() {
        Command::new("flatpak-spawn")
            .arg("--host")
            .arg(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    };
    status.map(|s| s.success()).unwrap_or(false)
}

fn systemctl_user(args: &[&str]) -> bool {
    let mut full: Vec<&str> = vec!["--user"];
    full.extend_from_slice(args);
    run_host_program("systemctl", &full)
}

fn try_systemctl_start() -> bool {
    systemctl_user(&["enable", "--now", "emobie-inputd.service"])
        || systemctl_user(&["start", "emobie-inputd.service"])
}

/// Best-effort restart after package/helper refresh (also used by updates).
pub fn try_restart_inputd_unit() -> bool {
    systemctl_user(&["daemon-reload"]);
    systemctl_user(&["try-restart", "emobie-inputd.service"])
        || systemctl_user(&["restart", "emobie-inputd.service"])
}

fn trusted_inputd_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("/usr/bin/emobie-inputd")];
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".local/bin/emobie-inputd"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("emobie-inputd"));
        }
    }
    paths
}

fn try_spawn_detached() -> bool {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return false;
    }
    for path in trusted_inputd_paths() {
        if !path.is_file() {
            continue;
        }
        if Command::new(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
        {
            return true;
        }
    }
    false
}

fn stop_all_helpers() {
    let _ = systemctl_user(&["stop", "emobie-inputd.service"]);
    // Wait for the unit to go inactive before touching sockets.
    for _ in 0..40 {
        if !systemctl_user(&["is-active", "--quiet", "emobie-inputd.service"]) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    // Only clear detached leftovers; prefer systemd for managed instances.
    let _ = run_host_program("pkill", &["-x", "emobie-inputd"]);
    for _ in 0..20 {
        if request(serde_json::json!({ "cmd": "status" })).is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

pub fn ensure_started() -> InputHelperStatus {
    if let Ok(resp) = request(serde_json::json!({ "cmd": "status" })) {
        // Enigo re-detects Wayland each inject — do not restart solely because
        // can_inject is false (burns heal and thrash on headless/early boot).
        return status_from_resp(resp);
    }
    let _ = super::super::bootstrap::try_bootstrap_host_helper();
    // Bootstrap may have started the host helper (Flatpak/AppImage); re-probe
    // before touching systemd — sandbox systemctl is not the host session.
    if let Ok(resp) = request(serde_json::json!({ "cmd": "status" })) {
        return status_from_resp(resp);
    }
    if try_systemctl_start() {
        if let Some(status) = wait_until_running(34) {
            return InputHelperStatus {
                detail: format!("started via systemd — {}", status.detail),
                ..status
            };
        }
    }
    if try_spawn_detached() {
        if let Some(status) = wait_until_running(34) {
            return InputHelperStatus {
                detail: format!("started helper — {}", status.detail),
                ..status
            };
        }
    }
    offline_status(
        "emobie-inputd not running — enable Expand to install the host helper automatically",
    )
}

/// Restart so can_listen re-opens devices after ACL/udev changes.
pub fn restart_helper() -> InputHelperStatus {
    stop_all_helpers();
    if try_systemctl_start() {
        if let Some(status) = wait_until_running(34) {
            return status;
        }
    }
    if try_spawn_detached() {
        if let Some(status) = wait_until_running(34) {
            return status;
        }
    }
    offline_status("could not restart emobie-inputd")
}
