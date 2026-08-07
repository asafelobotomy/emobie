//! On-disk preference reads used before the webview / plugin-store is ready.

use std::path::PathBuf;

const APP_IDENTIFIER: &str = "com.emobie.app";
const PREFERENCES_FILE: &str = "emobie-preferences.json";

pub fn store_path() -> Option<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })?;
    Some(data_home.join(APP_IDENTIFIER).join(PREFERENCES_FILE))
}

fn preferences_value() -> Option<serde_json::Value> {
    let path = store_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn pref_bool(key: &str) -> bool {
    preferences_value()
        .as_ref()
        .and_then(|value| value.get("preferences"))
        .and_then(|prefs| prefs.get(key))
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false)
}

/// Defaults to single-instance (false = enforce single instance).
pub fn allow_multiple_instances() -> bool {
    pref_bool("allowMultipleInstances")
}

pub fn start_minimized_to_tray() -> bool {
    pref_bool("startMinimizedToTray")
}
