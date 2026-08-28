//! On-disk preference reads used before the webview / plugin-store is ready,
//! plus a durable cross-install mirror under ~/.local/share/emobie/.

use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const APP_IDENTIFIER: &str = "io.github.asafelobotomy.emobie";
/// Pre-0.6.7 native store path (Tauri identifier was com.emobie.app).
const LEGACY_APP_IDENTIFIER: &str = "com.emobie.app";
const PREFERENCES_FILE: &str = "emobie-preferences.json";
const FLATPAK_APP_ID: &str = "io.github.asafelobotomy.emobie";
const DURABLE_FILE: &str = "preferences.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceSnapshot {
    pub source: String,
    pub preferences: Value,
}

pub fn store_path() -> Option<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })?;
    Some(data_home.join(APP_IDENTIFIER).join(PREFERENCES_FILE))
}

/// Stable host path shared by native, AppImage, deb/rpm, and Flatpak (via xdg-data/emobie).
pub fn durable_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("emobie")
            .join(DURABLE_FILE),
    )
}

fn host_store_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(APP_IDENTIFIER)
            .join(PREFERENCES_FILE),
    )
}

fn legacy_host_store_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(LEGACY_APP_IDENTIFIER)
            .join(PREFERENCES_FILE),
    )
}

fn flatpak_store_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".var")
            .join("app")
            .join(FLATPAK_APP_ID)
            .join("data")
            .join(APP_IDENTIFIER)
            .join(PREFERENCES_FILE),
    )
}

fn preferences_from_file(path: &PathBuf) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    if let Some(prefs) = value.get("preferences").cloned() {
        if prefs.is_object() {
            return Some(prefs);
        }
    }
    if value.is_object() {
        return Some(value);
    }
    None
}

fn preferences_value() -> Option<Value> {
    preferences_from_file(&store_path()?)
}

fn pref_bool(key: &str) -> bool {
    preferences_value()
        .as_ref()
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

pub fn pinned() -> bool {
    pref_bool("pinned")
}

/// Snapshots from durable mirror + host/Flatpak Tauri stores (deduped by path).
#[tauri::command]
pub fn load_preference_snapshots() -> Vec<PreferenceSnapshot> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    let candidates: [(&str, Option<PathBuf>); 5] = [
        ("durable", durable_path()),
        ("app-data", store_path()),
        ("host", host_store_path()),
        ("host-legacy", legacy_host_store_path()),
        ("flatpak", flatpak_store_path()),
    ];

    for (source, path) in candidates {
        let Some(path) = path else { continue };
        let key = path.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }
        if let Some(preferences) = preferences_from_file(&path) {
            out.push(PreferenceSnapshot {
                source: source.into(),
                preferences,
            });
        }
    }
    out
}

#[tauri::command]
pub fn save_durable_preferences(preferences: Value) -> Result<(), String> {
    if !preferences.is_object() {
        return Err("preferences must be a JSON object".into());
    }
    let path = durable_path().ok_or_else(|| "HOME is not set".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let wrapped = serde_json::json!({ "preferences": preferences });
    let body = serde_json::to_string_pretty(&wrapped).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}
