mod lifecycle;
mod socket;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Duration;

use super::{InputHelperStatus, InputMatch};

pub use lifecycle::{ensure_started, restart_helper, try_restart_inputd_unit};
pub use socket::{request, DaemonResponse};

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

#[cfg(target_os = "linux")]
pub fn native_inject_paste() -> Result<(), String> {
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
    match socket::request_with_timeout(
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
    match socket::request_with_timeout(
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
    use super::socket::{current_uid, trusted_socket_path};
    use std::path::Path;

    #[test]
    fn trusts_tmp_emobie_fallback() {
        let uid = current_uid();
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
