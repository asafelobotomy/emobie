//! Optional integration with the community-maintained "Focused Window D-Bus"
//! GNOME Shell extension (https://extensions.gnome.org/extension/5592/,
//! ~9k installs, actively maintained through GNOME Shell 50 as of writing).
//! Not bundled or auto-installed — when the user hasn't installed it, the
//! `Get` call below fails cleanly (unknown method/path on `org.gnome.Shell`)
//! and this returns `None`, same as any other undetectable session.

use zbus::blocking::Connection;
use zbus::proxy;

#[proxy(
    interface = "org.gnome.shell.extensions.FocusedWindow",
    default_service = "org.gnome.Shell",
    default_path = "/org/gnome/shell/extensions/FocusedWindow"
)]
trait FocusedWindow {
    fn get(&self) -> zbus::Result<String>;
}

pub fn active_window_class() -> Option<String> {
    let conn = Connection::session().ok()?;
    let proxy = FocusedWindowProxyBlocking::new(&conn).ok()?;
    let json = proxy.get().ok()?;
    let value: serde_json::Value = serde_json::from_str(&json).ok()?;
    value
        .get("wm_class")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}
