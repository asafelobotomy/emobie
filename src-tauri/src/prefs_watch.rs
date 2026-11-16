//! Tells the window when something other than the window changed its preferences file (the
//! `emobie` command line, the uQ Settings page): every change to the store file emits
//! `preferences-changed`. The front end reloads and ignores a change it made itself.

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

pub const CHANGED_EVENT: &str = "preferences-changed";

/// Keeps the watcher alive for the app's lifetime.
struct PrefsWatcher(#[allow(dead_code)] std::sync::Mutex<RecommendedWatcher>);

fn is_store_change(file: &Path, event: &notify::Event) -> bool {
    !event.kind.is_access() && event.paths.iter().any(|p| p == file)
}

/// Watches the folder of the store file (a save writes a new file and renames it over the old
/// one, so the file itself can't be watched). Without the folder nothing is watched and the
/// window reloads when it is focused again.
pub fn watch(app: &AppHandle) {
    let Some(file) = crate::prefs::store_path() else { return };
    let Some(dir) = file.parent().map(PathBuf::from) else { return };
    let handle = app.clone();
    let target = file.clone();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok_and(|ev| is_store_change(&target, &ev)) {
            let _ = handle.emit(CHANGED_EVENT, ());
        }
    });
    let Ok(mut watcher) = watcher else { return };
    if std::fs::create_dir_all(&dir).is_ok() && watcher.watch(&dir, RecursiveMode::NonRecursive).is_ok() {
        app.manage(PrefsWatcher(std::sync::Mutex::new(watcher)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{event::{AccessKind, CreateKind}, Event, EventKind};

    fn ev(kind: EventKind, path: &str) -> Event {
        Event::new(kind).add_path(PathBuf::from(path))
    }

    #[test]
    fn only_a_change_to_the_store_file_counts() {
        let file = Path::new("/d/emobie-preferences.json");
        assert!(is_store_change(file, &ev(EventKind::Create(CreateKind::File), "/d/emobie-preferences.json")));
        assert!(!is_store_change(file, &ev(EventKind::Create(CreateKind::File), "/d/emobie-preferences.json.1.tmp")));
        assert!(!is_store_change(file, &ev(EventKind::Access(AccessKind::Any), "/d/emobie-preferences.json")));
        assert!(!is_store_change(file, &ev(EventKind::Create(CreateKind::File), "/d/other.json")));
    }
}
