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
    #[allow(dead_code)]
    pub enabled: Option<bool>,
    pub error: Option<String>,
}

fn candidate_sockets() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(custom) = std::env::var("EMOBIE_INPUTD_SOCKET") {
        paths.push(PathBuf::from(custom));
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        paths.push(PathBuf::from(runtime).join("emobie/emobie-inputd.sock"));
    }
    paths.push(PathBuf::from("/run/emobie/emobie-inputd.sock"));
    paths
}

fn connect() -> Option<UnixStream> {
    for path in candidate_sockets() {
        if let Ok(stream) = UnixStream::connect(&path) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));
            return Some(stream);
        }
    }
    None
}

pub fn request(cmd: serde_json::Value) -> Result<DaemonResponse, String> {
    let mut stream = connect().ok_or_else(|| "emobie-inputd not running".to_string())?;
    let payload = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    writeln!(stream, "{payload}").map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    serde_json::from_str(line.trim()).map_err(|e| e.to_string())
}

pub fn offline_status(detail: &str) -> InputHelperStatus {
    let native_fallback =
        cfg!(target_os = "linux") && std::env::var_os("FLATPAK_ID").is_none();
    InputHelperStatus {
        daemon: false,
        can_inject: native_fallback,
        can_listen: false,
        detail: detail.to_string(),
        flatpak: false,
    }
}

fn status_from_resp(resp: DaemonResponse) -> InputHelperStatus {
    InputHelperStatus {
        daemon: true,
        can_inject: resp.can_inject,
        can_listen: resp.can_listen,
        detail: resp.detail,
        flatpak: false,
    }
}

pub fn status() -> InputHelperStatus {
    match request(serde_json::json!({ "cmd": "status" })) {
        Ok(resp) => status_from_resp(resp),
        Err(err) => offline_status(&err),
    }
}

fn wait_until_running(attempts: u32) -> Option<InputHelperStatus> {
    for _ in 0..attempts {
        thread::sleep(Duration::from_millis(150));
        if let Ok(resp) = request(serde_json::json!({ "cmd": "status" })) {
            return Some(status_from_resp(resp));
        }
    }
    None
}

fn systemctl_user(args: &[&str]) -> bool {
    Command::new("systemctl")
        .arg("--user")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn try_systemctl_start() -> bool {
    systemctl_user(&["enable", "--now", "emobie-inputd.service"])
        || systemctl_user(&["start", "emobie-inputd.service"])
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

pub fn ensure_started() -> InputHelperStatus {
    if let Ok(resp) = request(serde_json::json!({ "cmd": "status" })) {
        return status_from_resp(resp);
    }
    let _ = super::bootstrap::try_bootstrap_host_helper();
    if try_systemctl_start() {
        if let Some(status) = wait_until_running(20) {
            return InputHelperStatus {
                detail: format!("started via systemd — {}", status.detail),
                ..status
            };
        }
    }
    if try_spawn_detached() {
        if let Some(status) = wait_until_running(20) {
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
    let _ = systemctl_user(&["restart", "emobie-inputd.service"]);
    if wait_until_running(20).is_some() {
        return status();
    }
    for path in candidate_sockets() {
        let _ = std::fs::remove_file(&path);
    }
    thread::sleep(Duration::from_millis(200));
    if try_spawn_detached() {
        if let Some(status) = wait_until_running(20) {
            return status;
        }
    }
    ensure_started()
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
        Ok(resp) => Ok(status_from_resp(resp)),
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
    match request(serde_json::json!({
        "cmd": "sync_matches",
        "matches": matches,
    })) {
        Ok(resp) => Ok(status_from_resp(resp)),
        Err(err) => Ok(offline_status(&err)),
    }
}

pub fn inject_paste() -> Result<(), String> {
    let _ = ensure_started();
    match request(serde_json::json!({ "cmd": "inject_paste" })) {
        Ok(resp) if resp.ok => Ok(()),
        Ok(resp) => Err(resp.error.unwrap_or(resp.detail)),
        Err(_) => {
            if std::env::var_os("FLATPAK_ID").is_some() {
                return Err("emobie-inputd required inside Flatpak".into());
            }
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(native_inject_paste)) {
                Ok(result) => result,
                Err(_) => Err("Paste injection panicked".into()),
            }
        }
    }
}
