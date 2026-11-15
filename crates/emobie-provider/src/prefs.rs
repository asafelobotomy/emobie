//! Favourites, recents and the skin tone from emobie's preferences, read the way the app's
//! `src-tauri/src/prefs.rs` reads them: the Tauri store first, then the durable mirror
//! (which a Flatpak install also writes). Re-read only when the file changes.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

const APP_IDENTIFIER: &str = "io.github.asafelobotomy.emobie";
const PREFERENCES_FILE: &str = "emobie-preferences.json";
const DURABLE_FILE: &str = "preferences.json";
/// Preferences hold macros too, but stay far below this.
const MAX_BYTES: u64 = 4 * 1024 * 1024;
/// The app keeps at most a few dozen recents; favourites are unbounded, so cap both.
const MAX_LIST: usize = 256;
const MAX_ENTRY_BYTES: usize = 64;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Prefs {
    /// Hexcodes, most recently starred first (as the app stores them).
    pub favorites: Vec<String>,
    /// Emoji as copied (with skin tone), newest first.
    pub recents: Vec<String>,
    /// 0 (default yellow) to 5.
    pub skin_tone: u8,
}

fn strings(v: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(items)) = v else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= MAX_ENTRY_BYTES)
        .take(MAX_LIST)
        .map(str::to_string)
        .collect()
}

impl Prefs {
    /// From the store's text: `{"preferences": {...}}` or the bare object (durable mirror).
    pub fn parse(text: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        let prefs = match value.get("preferences") {
            Some(p) if p.is_object() => p,
            _ if value.is_object() => &value,
            _ => return None,
        };
        let skin_tone = prefs
            .get("skinTone")
            .and_then(Value::as_u64)
            .filter(|t| *t <= 5)
            .unwrap_or(0) as u8;
        Some(Self {
            favorites: strings(prefs.get("favorites")),
            recents: strings(prefs.get("recents")),
            skin_tone,
        })
    }
}

fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
}

/// Where the app keeps its preferences, in the order it reads them.
pub fn default_paths() -> Vec<PathBuf> {
    let Some(data) = data_home() else {
        return Vec::new();
    };
    vec![
        data.join(APP_IDENTIFIER).join(PREFERENCES_FILE),
        data.join("emobie").join(DURABLE_FILE),
    ]
}

/// Cached preferences, refreshed when the file in use changes.
pub struct PrefsSource {
    paths: Vec<PathBuf>,
    cache: Mutex<Option<(PathBuf, SystemTime, Prefs)>>,
}

fn read_capped(path: &Path) -> Option<(SystemTime, String)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    let mtime = meta.modified().ok()?;
    Some((mtime, std::fs::read_to_string(path).ok()?))
}

impl PrefsSource {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            cache: Mutex::new(None),
        }
    }

    /// The first readable preferences file, or defaults (no favourites, no recents).
    pub fn get(&self) -> Prefs {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        for path in &self.paths {
            let Ok(mtime) = std::fs::metadata(path).and_then(|m| m.modified()) else {
                continue;
            };
            if let Some((p, t, prefs)) = cache.as_ref() {
                if p == path && *t == mtime {
                    return prefs.clone();
                }
            }
            if let Some((mtime, text)) = read_capped(path) {
                if let Some(prefs) = Prefs::parse(&text) {
                    *cache = Some((path.clone(), mtime, prefs.clone()));
                    return prefs;
                }
            }
        }
        *cache = None;
        Prefs::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "emobie-provider-prefs-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_store_and_mirror_shapes() {
        let store = r#"{"preferences":{"favorites":["1F431"],"recents":["👋🏽","😀"],"skinTone":3}}"#;
        let p = Prefs::parse(store).unwrap();
        assert_eq!(p.favorites, ["1F431"]);
        assert_eq!(p.recents, ["👋🏽", "😀"]);
        assert_eq!(p.skin_tone, 3);
        let mirror = r#"{"favorites":["1F600"],"skinTone":9}"#;
        let p = Prefs::parse(mirror).unwrap();
        assert_eq!(p.favorites, ["1F600"]);
        assert_eq!(p.skin_tone, 0, "out-of-range tone is ignored");
        assert!(Prefs::parse("[1,2]").is_none());
        assert!(Prefs::parse("nope").is_none());
    }

    #[test]
    fn drops_bad_entries_and_caps_lists() {
        let long = "x".repeat(65);
        let many: Vec<String> = (0..300).map(|i| format!("\"{i}\"")).collect();
        let text = format!(
            r#"{{"favorites":[1,"",null,"{long}","1F600"],"recents":[{}]}}"#,
            many.join(",")
        );
        let p = Prefs::parse(&text).unwrap();
        assert_eq!(p.favorites, ["1F600"]);
        assert_eq!(p.recents.len(), MAX_LIST);
    }

    #[test]
    fn source_falls_back_and_follows_changes() {
        let dir = tempdir("source");
        let store = dir.join("store.json");
        let mirror = dir.join("mirror.json");
        let src = PrefsSource::new(vec![store.clone(), mirror.clone()]);
        assert_eq!(src.get(), Prefs::default());

        std::fs::write(&mirror, r#"{"favorites":["1F431"]}"#).unwrap();
        assert_eq!(src.get().favorites, ["1F431"]);

        std::fs::write(&store, "{broken").unwrap();
        assert_eq!(
            src.get().favorites,
            ["1F431"],
            "a broken store falls back to the mirror"
        );

        std::fs::write(&store, r#"{"preferences":{"favorites":["1F600"]}}"#).unwrap();
        // Move the mtime on explicitly: coarse timestamps could hide the rewrite.
        let f = std::fs::File::options().write(true).open(&store).unwrap();
        f.set_modified(SystemTime::now() + std::time::Duration::from_secs(2))
            .unwrap();
        assert_eq!(src.get().favorites, ["1F600"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
