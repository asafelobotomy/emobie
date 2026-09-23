//! When expansion must stay quiet: while the session is locked (keys typed
//! there are the user's password) and in excluded apps (password managers,
//! prompts), which only works where the focused app can be identified — see
//! `crate::focused_window`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedObjectPath;

static SESSION_LOCKED: AtomicBool = AtomicBool::new(false);
static EXCLUDED_APPS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn session_locked() -> bool {
    SESSION_LOCKED.load(Ordering::Acquire)
}

/// Case-insensitive; an entry matches when the app class contains it, so
/// `keepassxc` covers `org.keepassxc.KeePassXC`.
pub fn set_excluded_apps(apps: Vec<String>) {
    let cleaned = apps
        .into_iter()
        .map(|a| a.trim().to_lowercase())
        .filter(|a| !a.is_empty())
        .collect();
    if let Ok(mut guard) = EXCLUDED_APPS.lock() {
        *guard = cleaned;
    }
}

pub fn excluded_apps() -> Vec<String> {
    EXCLUDED_APPS.lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn has_excluded_apps() -> bool {
    EXCLUDED_APPS.lock().map(|g| !g.is_empty()).unwrap_or(false)
}

pub fn app_excluded(class: Option<&str>) -> bool {
    let Some(class) = class.map(str::to_lowercase) else {
        return false;
    };
    EXCLUDED_APPS
        .lock()
        .map(|list| list.iter().any(|entry| class.contains(entry.as_str())))
        .unwrap_or(false)
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.User",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/user/self"
)]
trait LoginUser {
    #[zbus(property)]
    fn display(&self) -> zbus::Result<(String, OwnedObjectPath)>;
}

#[zbus::proxy(interface = "org.freedesktop.login1.Session", default_service = "org.freedesktop.login1")]
trait LoginSession {
    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
}

/// Best-effort: without logind (or a desktop that sets LockedHint) this
/// thread exits and only the excluded-apps list applies.
pub fn spawn_lock_watch() {
    std::thread::spawn(|| {
        if let Err(err) = watch_lock() {
            eprintln!("emobie-inputd: screen-lock watch unavailable ({err})");
        }
    });
}

fn watch_lock() -> zbus::Result<()> {
    let conn = Connection::system()?;
    // `session/auto` fails from a systemd user service (not in a session
    // scope); the user's graphical session is its `Display`.
    let (_, path) = LoginUserProxyBlocking::new(&conn)?.display()?;
    let session = LoginSessionProxyBlocking::builder(&conn).path(path)?.build()?;
    SESSION_LOCKED.store(session.locked_hint().unwrap_or(false), Ordering::Release);
    for change in session.receive_locked_hint_changed() {
        if let Ok(locked) = change.get() {
            SESSION_LOCKED.store(locked, Ordering::Release);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusion_is_case_insensitive_substring() {
        set_excluded_apps(vec![" KeePassXC ".into(), String::new()]);
        assert!(app_excluded(Some("org.keepassxc.KeePassXC")));
        assert!(!app_excluded(Some("org.gnome.TextEditor")));
        assert!(!app_excluded(None));
        set_excluded_apps(Vec::new());
        assert!(!has_excluded_apps());
    }
}
