//! `_NET_ACTIVE_WINDOW` + `WM_CLASS` via a fresh, short-lived X11 connection.
//! Works on plain X11 sessions and, via XWayland, sees XWayland-backed apps
//! under a Wayland session too — but not native-Wayland toolkit windows,
//! which have no X11 window at all. See `focused_window::detect_class` for
//! how that's ordered against the GNOME extension path.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt};

fn intern(conn: &impl Connection, name: &str) -> Option<Atom> {
    conn.intern_atom(false, name.as_bytes())
        .ok()?
        .reply()
        .ok()
        .map(|reply| reply.atom)
}

pub fn active_window_class() -> Option<String> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen_num)?.root;

    let net_active_window = intern(&conn, "_NET_ACTIVE_WINDOW")?;
    let wm_class = intern(&conn, "WM_CLASS")?;

    let active = conn
        .get_property(false, root, net_active_window, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let window = active.value32()?.next()?;
    if window == 0 {
        return None;
    }

    let class_prop = conn
        .get_property(false, window, wm_class, AtomEnum::STRING, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    // WM_CLASS is two NUL-terminated strings: instance, then class. Prefer
    // the class (second part) — it's the stable per-application identifier;
    // fall back to the instance if the class part is somehow empty.
    let parts: Vec<&[u8]> = class_prop
        .value
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .collect();
    let class = parts.get(1).or_else(|| parts.first())?;
    String::from_utf8(class.to_vec()).ok()
}
