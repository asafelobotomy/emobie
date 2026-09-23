//! Start/stop emobie-inputd via systemd or a detached binary.

use super::socket::{request, DaemonResponse};
use super::{offline_status, status_from_resp};
use crate::input_helper::InputHelperStatus;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Cap on the whole bootstrap/start chain below. It shells out to
/// tar/bash/systemctl (and `flatpak-spawn --host` for those under Flatpak)
/// with no per-call timeout of their own, so a wedged host command (e.g. a
/// stuck portal prompt) must not be able to hang callers indefinitely —
/// callers already run this off the UI thread, but should still get an
/// answer in bounded time.
/// Serializes start/restart. `ensure_started` is reachable from several
/// commands at once (status polling, sync, options, Grant); without this two
/// callers can both bootstrap/`systemctl start`, or one can start the daemon
/// while another is stopping it. Waiters re-probe status once they get the
/// lock, so a helper started meanwhile is simply reused.
static START_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const ENSURE_STARTED_BUDGET: Duration = Duration::from_secs(12);

pub fn ensure_started() -> InputHelperStatus {
    let (tx, rx) = std::sync::mpsc::channel();
    // Detached: if this outruns the budget it keeps trying in the background
    // (harmless — the daemon may still end up running for the next call) and
    // is simply dropped once it eventually finishes.
    thread::spawn(move || {
        let _ = tx.send(ensure_started_inner());
    });
    rx.recv_timeout(ENSURE_STARTED_BUDGET).unwrap_or_else(|_| {
        offline_status("emobie-inputd start timed out — a host command may be stuck")
    })
}

/// The running-helper version check happens once per app session.
static VERSION_CHECKED: AtomicBool = AtomicBool::new(false);

fn helper_outdated(resp: &DaemonResponse) -> bool {
    match resp.version.as_deref() {
        Some(running) => {
            super::super::bootstrap::version_lt(running, env!("CARGO_PKG_VERSION"))
                .unwrap_or(false)
        }
        None => true,
    }
}

/// A running helper answers `status`, so the bootstrap (which does the
/// version comparison) would otherwise never run after an app update.
/// The bootstrap replaces the binary and restarts the unit.
fn upgrade_running_helper(current: DaemonResponse) -> InputHelperStatus {
    if !super::super::bootstrap::try_bootstrap_host_helper() {
        return status_from_resp(current);
    }
    wait_until_running(34)
        .unwrap_or_else(|| offline_status("emobie-inputd did not come back after updating it"))
}

fn ensure_started_inner() -> InputHelperStatus {
    let _guard = START_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(resp) = request(serde_json::json!({ "cmd": "status" })) {
        if !VERSION_CHECKED.swap(true, Ordering::AcqRel) && helper_outdated(&resp) {
            return upgrade_running_helper(resp);
        }
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
    let _guard = START_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

#[cfg(test)]
mod tests {
    use super::{helper_outdated, DaemonResponse};

    fn resp(version: Option<&str>) -> DaemonResponse {
        let mut value = serde_json::json!({
            "ok": true,
            "can_inject": true,
            "can_listen": false,
            "detail": "",
            "error": null,
        });
        if let Some(v) = version {
            value["version"] = v.into();
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn helper_without_version_is_outdated() {
        assert!(helper_outdated(&resp(None)));
    }

    #[test]
    fn compares_running_version_to_app() {
        assert!(helper_outdated(&resp(Some("0.0.1"))));
        assert!(!helper_outdated(&resp(Some(env!("CARGO_PKG_VERSION")))));
        assert!(!helper_outdated(&resp(Some("99.0.0"))));
    }
}
