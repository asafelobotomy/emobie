use super::{InputHelperStatus, InputMatch};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Deserialize)]
pub struct DaemonResponse {
    pub ok: bool,
    pub can_inject: bool,
    pub can_listen: bool,
    pub detail: String,
    pub error: Option<String>,
}

fn current_uid() -> u32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|u| u.parse().ok())
        })
        .unwrap_or(0)
}

fn tmp_emobie_socket() -> PathBuf {
    PathBuf::from(format!("/tmp/emobie-{}/emobie-inputd.sock", current_uid()))
}

fn candidate_sockets() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(custom) = std::env::var("EMOBIE_INPUTD_SOCKET") {
        let path = PathBuf::from(&custom);
        if trusted_socket_path(&path) {
            paths.push(path);
        }
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        paths.push(PathBuf::from(runtime).join("emobie/emobie-inputd.sock"));
    }
    paths.push(PathBuf::from("/run/emobie/emobie-inputd.sock"));
    paths.push(tmp_emobie_socket());
    paths
}

fn trusted_socket_path(path: &std::path::Path) -> bool {
    // Keep in sync with crates/emobie-inputd/src/socket_path.rs::is_trusted.
    if path.file_name().and_then(|n| n.to_str()) != Some("emobie-inputd.sock") {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if parent == std::path::Path::new("/run/emobie") {
        return true;
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        if parent == std::path::Path::new(&runtime).join("emobie") {
            return true;
        }
    }
    let tmp_fallback = PathBuf::from(format!("/tmp/emobie-{}", current_uid()));
    if parent == tmp_fallback.as_path() {
        return true;
    }
    false
}

fn connect_with_timeout(timeout: Duration) -> Option<UnixStream> {
    for path in candidate_sockets() {
        if let Ok(stream) = UnixStream::connect(&path) {
            let _ = stream.set_read_timeout(Some(timeout));
            let _ = stream.set_write_timeout(Some(timeout));
            return Some(stream);
        }
    }
    None
}

pub fn request(cmd: serde_json::Value) -> Result<DaemonResponse, String> {
    request_with_timeout(cmd, Duration::from_millis(800))
}

fn request_with_timeout(
    cmd: serde_json::Value,
    timeout: Duration,
) -> Result<DaemonResponse, String> {
    let mut stream =
        connect_with_timeout(timeout).ok_or_else(|| "emobie-inputd not running".to_string())?;
    let payload = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    writeln!(stream, "{payload}").map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    serde_json::from_str(line.trim()).map_err(|e| e.to_string())
}

pub fn offline_status(detail: &str) -> InputHelperStatus {
    InputHelperStatus {
        daemon: false,
        can_inject: false,
        can_listen: false,
        detail: detail.to_string(),
        flatpak: false,
        access_configured: false,
    }
}

fn status_from_resp(resp: DaemonResponse) -> InputHelperStatus {
    InputHelperStatus {
        daemon: true,
        can_inject: resp.can_inject,
        can_listen: resp.can_listen,
        detail: resp.detail,
        flatpak: false,
        access_configured: false,
    }
}

pub fn status() -> InputHelperStatus {
    match request(serde_json::json!({ "cmd": "status" })) {
        Ok(resp) => status_from_resp(resp),
        Err(err) => offline_status(&err),
    }
}

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
    let _ = super::bootstrap::try_bootstrap_host_helper();
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

#[cfg(target_os = "linux")]
pub fn native_inject_paste() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::panic::{catch_unwind, AssertUnwindSafe};
    match catch_unwind(AssertUnwindSafe(|| {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| e.to_string())?;
        Ok(())
    })) {
        Ok(inner) => inner,
        Err(_) => Err("input injection backend panicked".into()),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn native_inject_paste() -> Result<(), String> {
    Err("paste injection is only supported on Linux".into())
}

pub fn set_enabled(enabled: bool) -> Result<InputHelperStatus, String> {
    if enabled {
        let _ = ensure_started();
    }
    match request(serde_json::json!({ "cmd": "set_enabled", "enabled": enabled })) {
        Ok(resp) if resp.ok => Ok(status_from_resp(resp)),
        Ok(resp) => {
            if enabled {
                Err(resp.error.unwrap_or(resp.detail))
            } else {
                Ok(status_from_resp(resp))
            }
        }
        Err(err) => {
            if enabled {
                Err(err)
            } else {
                Ok(offline_status(&err))
            }
        }
    }
}

pub fn sync_matches(matches: Vec<InputMatch>) -> Result<InputHelperStatus, String> {
    let _ = ensure_started();
    match request_with_timeout(
        serde_json::json!({
            "cmd": "sync_matches",
            "matches": matches,
        }),
        Duration::from_secs(8),
    ) {
        Ok(resp) if resp.ok => Ok(status_from_resp(resp)),
        Ok(resp) => Err(resp.error.unwrap_or(resp.detail)),
        Err(err) => Err(err),
    }
}

pub fn inject_paste() -> Result<(), String> {
    let _ = ensure_started();
    match request_with_timeout(
        serde_json::json!({ "cmd": "inject_paste" }),
        Duration::from_secs(3),
    ) {
        Ok(resp) if resp.ok => Ok(()),
        Ok(resp) => Err(resp.error.unwrap_or(resp.detail)),
        Err(_) => {
            if std::env::var_os("FLATPAK_ID").is_some() {
                return Err("emobie-inputd required inside Flatpak".into());
            }
            native_inject_paste()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::trusted_socket_path;
    use std::path::Path;

    #[test]
    fn trusts_tmp_emobie_fallback() {
        let uid = super::current_uid();
        let path = format!("/tmp/emobie-{uid}/emobie-inputd.sock");
        assert!(trusted_socket_path(Path::new(&path)));
    }

    #[test]
    fn rejects_untrusted_socket() {
        assert!(!trusted_socket_path(Path::new(
            "/tmp/evil/emobie-inputd.sock"
        )));
    }
}
