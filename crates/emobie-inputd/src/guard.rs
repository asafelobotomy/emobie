//! When expansion must stay quiet: while the session is locked (keys typed
//! there are the user's password), while it is not the seat's active session
//! (input devices are global, so keys then belong to another user or the
//! login screen), and in excluded apps (password managers, prompts), which
//! only works where the focused app can be identified — see
//! `crate::focused_window`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedObjectPath;

static SESSION_LOCKED: AtomicBool = AtomicBool::new(false);
static SESSION_ACTIVE: AtomicBool = AtomicBool::new(true);
/// Set once the session watch has stopped (never started, or logind's signal
/// stream ended). Not set while it is still connecting at startup.
static SESSION_WATCH_FAILED: AtomicBool = AtomicBool::new(false);
static EXCLUDED_APPS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn session_locked() -> bool {
    SESSION_LOCKED.load(Ordering::Acquire)
}

/// False while another session (another user, or the login screen) owns the
/// seat. Keys read then are not ours, and anything injected would land there.
pub fn session_active() -> bool {
    SESSION_ACTIVE.load(Ordering::Acquire)
}

/// Locked or not the active session: typed keys must not be matched.
pub fn session_paused() -> bool {
    session_locked() || !session_active()
}

/// True when the lock-screen / session-switch pause is not protecting typed
/// keys, so the app can say so instead of the failure living only in the
/// journal.
pub fn session_watch_failed() -> bool {
    SESSION_WATCH_FAILED.load(Ordering::Acquire)
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
    #[zbus(property)]
    fn active(&self) -> zbus::Result<bool>;
}

/// Best-effort: without logind (or a desktop that sets LockedHint) these
/// watches exit, `session_watch_failed` turns true, and only the excluded-apps
/// list applies.
pub fn spawn_lock_watch() {
    std::thread::spawn(|| {
        let session = match session_proxy() {
            Ok(session) => session,
            Err(err) => {
                eprintln!("emobie-inputd: session watch unavailable ({err})");
                SESSION_WATCH_FAILED.store(true, Ordering::Release);
                return;
            }
        };
        SESSION_LOCKED.store(session.locked_hint().unwrap_or(false), Ordering::Release);
        SESSION_ACTIVE.store(session.active().unwrap_or(true), Ordering::Release);
        let active_session = session.clone();
        std::thread::spawn(move || {
            for change in active_session.receive_active_changed() {
                if let Ok(active) = change.get() {
                    SESSION_ACTIVE.store(active, Ordering::Release);
                }
            }
            watch_ended("active-session");
        });
        for change in session.receive_locked_hint_changed() {
            if let Ok(locked) = change.get() {
                SESSION_LOCKED.store(locked, Ordering::Release);
            }
        }
        watch_ended("screen-lock");
    });
}

fn watch_ended(what: &str) {
    eprintln!("emobie-inputd: {what} watch ended (logind signal stream closed)");
    SESSION_WATCH_FAILED.store(true, Ordering::Release);
}

fn session_proxy() -> zbus::Result<LoginSessionProxyBlocking<'static>> {
    let conn = Connection::system()?;
    // `session/auto` fails from a systemd user service (not in a session
    // scope); the user's graphical session is its `Display`.
    let (_, path) = LoginUserProxyBlocking::new(&conn)?.display()?;
    LoginSessionProxyBlocking::builder(&conn).path(path)?.build()
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
