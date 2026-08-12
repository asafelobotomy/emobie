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
        paths.push(PathBuf::from("/run/emobie/emobie-inputd.sock"));
        if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
            paths.push(PathBuf::from(runtime).join("emobie/emobie-inputd.sock"));
        }
        paths.push(PathBuf::from("/tmp/emobie/emobie-inputd.sock"));
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

    pub fn status() -> InputHelperStatus {
        match request(serde_json::json!({ "cmd": "status" })) {
            Ok(resp) => InputHelperStatus {
                daemon: true,
                can_inject: resp.can_inject,
                can_listen: resp.can_listen,
                detail: resp.detail,
            },
            Err(err) => offline_status(&err),
        }
    }

    pub fn set_enabled(enabled: bool) -> Result<InputHelperStatus, String> {
        match request(serde_json::json!({ "cmd": "set_enabled", "enabled": enabled })) {
            Ok(resp) => Ok(InputHelperStatus {
                daemon: true,
                can_inject: resp.can_inject,
                can_listen: resp.can_listen,
                detail: resp.detail,
            }),
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
            Ok(resp) => Ok(InputHelperStatus {
                daemon: true,
                can_inject: resp.can_inject,
                can_listen: resp.can_listen,
                detail: resp.detail,
            }),
            Err(err) => Ok(offline_status(&err)),
        }
    }

    pub fn inject_paste() -> Result<(), String> {
        match request(serde_json::json!({ "cmd": "inject_paste" })) {
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
