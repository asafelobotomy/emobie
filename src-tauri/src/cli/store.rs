//! Reads and writes emobie's preferences file the way the app does: the Tauri store
//! (`{"preferences": {...}}`) is authoritative, the durable mirror is refreshed after every
//! write, and a running app notices the change on its own (see `prefs_watch.rs`).

use crate::prefs;
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Preferences hold macros too, but stay far below this.
const MAX_BYTES: u64 = 4 * 1024 * 1024;

pub type Prefs = Map<String, Value>;

pub struct Loaded {
    pub prefs: Prefs,
    /// The file the preferences came from; None when emobie has never saved any.
    pub path: Option<PathBuf>,
}

/// Where the app's own store is: the first of the native, legacy-free host and Flatpak paths
/// that exists, else the native one (so a first write creates it where the app will look).
pub fn primary_path() -> Option<PathBuf> {
    let candidates = [prefs::store_path(), prefs::host_store_path(), prefs::flatpak_store_path()];
    candidates
        .iter()
        .flatten()
        .find(|p| p.is_file())
        .cloned()
        .or_else(|| candidates.into_iter().next().flatten())
}

fn read_json(path: &Path) -> Result<Option<Value>, String> {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(format!("{} is not a readable preferences file", path.display()));
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| format!("{} is not valid JSON: {e}", path.display()))
}

fn prefs_of(root: &Value) -> Prefs {
    match root.get("preferences") {
        Some(Value::Object(p)) => p.clone(),
        _ => Prefs::new(),
    }
}

/// The saved preferences: the store, else the durable mirror, else nothing saved yet.
pub fn load() -> Result<Loaded, String> {
    if let Some(path) = primary_path() {
        if let Some(root) = read_json(&path)? {
            return Ok(Loaded { prefs: prefs_of(&root), path: Some(path) });
        }
    }
    if let Some(path) = prefs::durable_path() {
        if let Some(root) = read_json(&path)? {
            // The mirror is a hint for a fresh store: reading is fine, the next write recreates it.
            return Ok(Loaded { prefs: prefs_of(&root), path: Some(path) });
        }
    }
    Ok(Loaded { prefs: Prefs::new(), path: None })
}

fn write_atomic(path: &Path, body: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("no parent directory")?;
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(parent).map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let result = (|| -> std::io::Result<()> {
        let mut file = opts.open(&tmp)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("could not write {}: {e}", path.display())
    })
}

/// Saves `next` into the store (keeping any other top-level keys the file has) and refreshes
/// the durable mirror. Returns the path written.
pub fn save(next: &Prefs) -> Result<PathBuf, String> {
    let path = primary_path().ok_or("HOME is not set")?;
    let mut root = match read_json(&path)? {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    root.insert("preferences".into(), Value::Object(next.clone()));
    let body = serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| e.to_string())?;
    write_atomic(&path, &body)?;
    // A write newer than anything the app mirrored: the app uses the time in ms as its rev too.
    let rev = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(1)
        .max(1);
    // A mirror that can't be written (a Flatpak without xdg-data/emobie) doesn't fail the save.
    let _ = prefs::save_durable_preferences(Value::Object(next.clone()), rev);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::testenv::with_home;

    #[test]
    fn nothing_saved_reads_as_empty() {
        with_home("empty", |_| {
            let loaded = load().unwrap();
            assert!(loaded.prefs.is_empty());
            assert!(loaded.path.is_none());
        });
    }

    #[test]
    fn save_writes_the_store_and_the_mirror_and_keeps_other_keys() {
        with_home("save", |home| {
            let store = home.join(".local/share/io.github.asafelobotomy.emobie/emobie-preferences.json");
            fs::create_dir_all(store.parent().unwrap()).unwrap();
            fs::write(&store, r#"{"preferences":{"theme":"dark"},"other":1}"#).unwrap();

            let mut p = load().unwrap().prefs;
            assert_eq!(p["theme"], "dark");
            p.insert("theme".into(), "light".into());
            let path = save(&p).unwrap();
            assert_eq!(path, store);

            let root: Value = serde_json::from_str(&fs::read_to_string(&store).unwrap()).unwrap();
            assert_eq!(root["preferences"]["theme"], "light");
            assert_eq!(root["other"], 1);
            let mirror: Value = serde_json::from_str(
                &fs::read_to_string(home.join(".local/share/emobie/preferences.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(mirror["preferences"]["theme"], "light");
            assert!(mirror["writeRev"].as_u64().unwrap() > 0);
        });
    }

    #[test]
    fn a_broken_store_is_an_error_not_an_empty_reset() {
        with_home("broken", |home| {
            let store = home.join(".local/share/io.github.asafelobotomy.emobie/emobie-preferences.json");
            fs::create_dir_all(store.parent().unwrap()).unwrap();
            fs::write(&store, "{not json").unwrap();
            assert!(load().is_err());
        });
    }
}
