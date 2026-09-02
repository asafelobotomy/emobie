//! Host emobie-inputd client: status, ensure-started, paste inject, access setup.

use serde::{Deserialize, Serialize};

#[cfg(unix)]
mod access;
#[cfg(unix)]
pub mod bootstrap;
#[cfg(unix)]
pub mod unix;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputHelperStatus {
    pub daemon: bool,
    pub can_inject: bool,
    pub can_listen: bool,
    pub detail: String,
    /// True when running inside a Flatpak sandbox.
    #[serde(default)]
    pub flatpak: bool,
    /// True when group `emobie-input` and system udev rules are present.
    /// Distinct from `can_listen`, which can be true via a temporary ACL or
    /// orphaned GID even when permanent Grant config is missing.
    #[serde(default)]
    pub access_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMatch {
    pub trigger: String,
    pub expansion: String,
  #[serde(default = "default_mode")]
  pub mode: String,
}

fn default_mode() -> String {
    "space".into()
}

#[cfg(not(unix))]
fn offline_linux_only() -> InputHelperStatus {
    InputHelperStatus {
        daemon: false,
        can_inject: false,
        can_listen: false,
        detail: "Input helper is Linux-only.".into(),
        flatpak: false,
        access_configured: false,
    }
}

#[tauri::command]
pub fn input_helper_status() -> InputHelperStatus {
    #[cfg(unix)]
    {
        return access::with_flatpak_flag(unix::status());
    }
    #[cfg(not(unix))]
    {
        offline_linux_only()
    }
}

#[tauri::command]
pub fn input_helper_ensure_started() -> InputHelperStatus {
    #[cfg(unix)]
    {
        return access::with_flatpak_flag(unix::ensure_started());
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
        return unix::set_enabled(enabled).map(access::with_flatpak_flag);
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
        return unix::sync_matches(matches).map(access::with_flatpak_flag);
    }
    #[cfg(not(unix))]
    {
        let _ = matches;
        Ok(offline_linux_only())
    }
}

#[tauri::command]
pub fn input_helper_inject_paste() -> Result<(), String> {
    #[cfg(unix)]
    {
        return unix::inject_paste();
    }
    #[cfg(not(unix))]
    {
        Err("Input helper is Linux-only.".into())
    }
}

#[tauri::command]
pub fn input_helper_run_access_setup() -> Result<InputHelperStatus, String> {
    #[cfg(unix)]
    {
        return access::run_access_setup();
    }
    #[cfg(not(unix))]
    {
        Err("Input helper is Linux-only.".into())
    }
}
