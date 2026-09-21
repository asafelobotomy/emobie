//! Host emobie-inputd client: status, ensure-started, paste inject, access setup.

use serde::{Deserialize, Serialize};

#[cfg(unix)]
mod access;
#[cfg(unix)]
pub mod bootstrap;
#[cfg(unix)]
mod bootstrap_tar;
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
    /// In-flight expand jobs holding listen suppress (debug).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_jobs: Option<usize>,
    /// Whether clipboard restore after paste is enabled (default false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_clipboard: Option<bool>,
    /// Last expand insert backend: keys | ei | wl-copy | arboard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_inject_backend: Option<String>,
    /// "auto" (default, focused-window detection), "ctrl_v", "shift_insert",
    /// or "ctrl_shift_v" — see crate::paste_chord in emobie-inputd.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paste_chord: Option<String>,
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
        suppress_jobs: None,
        restore_clipboard: None,
        last_inject_backend: None,
        paste_chord: None,
    }
}

/// Run blocking helper work (sockets, `flatpak-spawn`, `pkexec`) off the main
/// thread — synchronous Tauri commands execute *on* it, so a slow host command
/// would freeze the window.
async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| format!("input helper task failed: {err}"))
}

fn task_failed(err: String) -> InputHelperStatus {
    InputHelperStatus {
        daemon: false,
        can_inject: false,
        can_listen: false,
        detail: err,
        flatpak: false,
        access_configured: false,
        suppress_jobs: None,
        restore_clipboard: None,
        last_inject_backend: None,
        paste_chord: None,
    }
}

#[tauri::command]
pub async fn input_helper_status() -> InputHelperStatus {
    blocking(|| {
        #[cfg(unix)]
        {
            access::with_flatpak_flag(unix::status())
        }
        #[cfg(not(unix))]
        {
            offline_linux_only()
        }
    })
    .await
    .unwrap_or_else(task_failed)
}

#[tauri::command]
pub async fn input_helper_ensure_started() -> InputHelperStatus {
    blocking(|| {
        #[cfg(unix)]
        {
            access::with_flatpak_flag(unix::ensure_started())
        }
        #[cfg(not(unix))]
        {
            offline_linux_only()
        }
    })
    .await
    .unwrap_or_else(task_failed)
}

#[tauri::command]
pub async fn input_helper_set_enabled(enabled: bool) -> Result<InputHelperStatus, String> {
    blocking(move || {
        #[cfg(unix)]
        {
            unix::set_enabled(enabled).map(access::with_flatpak_flag)
        }
        #[cfg(not(unix))]
        {
            let _ = enabled;
            Err("Input helper is Linux-only.".to_string())
        }
    })
    .await?
}

#[tauri::command]
pub async fn input_helper_sync_matches(
    matches: Vec<InputMatch>,
) -> Result<InputHelperStatus, String> {
    blocking(move || {
        #[cfg(unix)]
        {
            unix::sync_matches(matches).map(access::with_flatpak_flag)
        }
        #[cfg(not(unix))]
        {
            let _ = matches;
            Ok(offline_linux_only())
        }
    })
    .await?
}

#[tauri::command]
pub async fn input_helper_set_options(
    restore_clipboard: Option<bool>,
    paste_chord: Option<String>,
) -> Result<InputHelperStatus, String> {
    blocking(move || {
        #[cfg(unix)]
        {
            unix::set_options(restore_clipboard, paste_chord).map(access::with_flatpak_flag)
        }
        #[cfg(not(unix))]
        {
            let _ = (restore_clipboard, paste_chord);
            Err("Input helper is Linux-only.".to_string())
        }
    })
    .await?
}

#[tauri::command]
pub async fn input_helper_inject_paste() -> Result<(), String> {
    blocking(|| {
        #[cfg(unix)]
        {
            unix::inject_paste()
        }
        #[cfg(not(unix))]
        {
            Err("Input helper is Linux-only.".to_string())
        }
    })
    .await?
}

#[tauri::command]
pub async fn input_helper_run_access_setup() -> Result<InputHelperStatus, String> {
    blocking(|| {
        #[cfg(unix)]
        {
            access::run_access_setup()
        }
        #[cfg(not(unix))]
        {
            Err("Input helper is Linux-only.".to_string())
        }
    })
    .await?
}
