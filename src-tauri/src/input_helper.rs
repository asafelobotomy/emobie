//! Host emobie-inputd client: status, ensure-started, paste inject.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputHelperStatus {
    pub daemon: bool,
    pub can_inject: bool,
    pub can_listen: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMatch {
    pub trigger: String,
    pub expansion: String,
}

#[cfg(unix)]
mod unix_helper {
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
        // Legacy path kept for older installs only.
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
        }
    }

    fn status_from_resp(resp: DaemonResponse) -> InputHelperStatus {
        InputHelperStatus {
            daemon: true,
            can_inject: resp.can_inject,
            can_listen: resp.can_listen,
            detail: resp.detail,
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

    fn try_systemctl_start() -> bool {
        let enable = Command::new("systemctl")
            .args(["--user", "enable", "--now", "emobie-inputd.service"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if enable.map(|s| s.success()).unwrap_or(false) {
            return true;
        }
        Command::new("systemctl")
            .args(["--user", "start", "emobie-inputd.service"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn trusted_inputd_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        paths.push(PathBuf::from("/usr/bin/emobie-inputd"));
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
            let Ok(child) = Command::new(&path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            else {
                continue;
            };
            let _ = child.id();
            return true;
        }
        false
    }

    pub fn ensure_started() -> InputHelperStatus {
        if let Ok(resp) = request(serde_json::json!({ "cmd": "status" })) {
            return status_from_resp(resp);
        }

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
            "emobie-inputd not running — install with packaging/install-inputd-user.sh",
        )
    }

    #[cfg(target_os = "linux")]
    pub fn native_inject_paste() -> Result<(), String> {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};
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

    fn access_setup_scripts() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        paths.push(PathBuf::from("/usr/share/emobie/setup-input-access.sh"));
        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                PathBuf::from(home).join(".local/share/emobie/setup-input-access.sh"),
            );
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                paths.push(dir.join("setup-input-access.sh"));
                paths.push(dir.join("../../../packaging/setup-input-access.sh"));
            }
        }
        paths
    }

    pub fn run_access_setup() -> Result<String, String> {
        if std::env::var_os("FLATPAK_ID").is_some() {
            return Err(
                "Flatpak cannot run host pkexec — install the helper on the host and run setup-input-access.sh there."
                    .into(),
            );
        }
        let script = access_setup_scripts()
            .into_iter()
            .find(|path| path.is_file())
            .ok_or_else(|| {
                "setup-input-access.sh not found — install the emobie package or run packaging/setup-input-access.sh".to_string()
            })?;
        let user = std::env::var("USER").unwrap_or_default();
        let status = Command::new("pkexec")
            .args([
                "env",
                &format!("SUDO_USER={user}"),
                "bash",
                script.to_str().unwrap_or(""),
            ])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok("Keyboard access configured. Log out and back in, then enable Expand as you type.".into())
        } else {
            Err("Setup was cancelled or failed.".into())
        }
    }
}

#[tauri::command]
pub fn input_helper_status() -> InputHelperStatus {
    #[cfg(unix)]
    {
        return unix_helper::status();
    }
    #[cfg(not(unix))]
    {
        InputHelperStatus {
            daemon: false,
            can_inject: false,
            can_listen: false,
            detail: "Input helper is Linux-only.".into(),
        }
    }
}

#[tauri::command]
pub fn input_helper_ensure_started() -> InputHelperStatus {
    #[cfg(unix)]
    {
        return unix_helper::ensure_started();
    }
    #[cfg(not(unix))]
    {
        input_helper_status()
    }
}

#[tauri::command]
pub fn input_helper_set_enabled(enabled: bool) -> Result<InputHelperStatus, String> {
    #[cfg(unix)]
    {
        return unix_helper::set_enabled(enabled);
    }
    #[cfg(not(unix))]
    {
        let _ = enabled;
        Err("Input helper is Linux-only.".into())
    }
}

#[tauri::command]
pub fn input_helper_sync_matches(matches: Vec<InputMatch>) -> Result<InputHelperStatus, String> {
    #[cfg(unix)]
    {
        return unix_helper::sync_matches(matches);
    }
    #[cfg(not(unix))]
    {
        let _ = matches;
        Ok(InputHelperStatus {
            daemon: false,
            can_inject: false,
            can_listen: false,
            detail: "Input helper is Linux-only.".into(),
        })
    }
}

#[tauri::command]
pub fn input_helper_inject_paste() -> Result<(), String> {
    #[cfg(unix)]
    {
        return unix_helper::inject_paste();
    }
    #[cfg(not(unix))]
    {
        Err("Input helper is Linux-only.".into())
    }
}

#[tauri::command]
pub fn input_helper_run_access_setup() -> Result<String, String> {
    #[cfg(unix)]
    {
        return unix_helper::run_access_setup();
    }
    #[cfg(not(unix))]
    {
        Err("Input helper is Linux-only.".into())
    }
}
