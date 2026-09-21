//! Best-effort focused-window class detection, used only to pick a paste
//! chord (see `crate::paste_chord`) — never a hard requirement. Every path
//! here degrades to `None` (falls back to the default Ctrl+V chord) rather
//! than erroring or blocking, so a slow/missing X server or D-Bus service
//! never delays a paste.

mod gnome;
mod x11;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Upper bound on one focused-window lookup before falling back to Ctrl+V.
const LOOKUP_TIMEOUT: Duration = Duration::from_millis(300);
static LOOKUP_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// On a Wayland session, tries the optional "Focused Window D-Bus" GNOME
/// Shell extension (https://extensions.gnome.org/extension/5592/) first —
/// it's authoritative for native-Wayland clients, whereas XWayland's
/// `_NET_ACTIVE_WINDOW` can report a stale XWayland-only window once focus
/// has moved to a native-Wayland client, since XWayland has no visibility
/// into native clients at all. X11 (or XWayland as a fallback) covers pure
/// X11 sessions and XWayland-backed apps that the extension path can't see —
/// the same X11-first fallback Espanso's own X11AppInfoProvider uses there.
/// Neither is a hard dependency: without a GNOME extension installed and
/// without an X server, this returns `None` and paste_chord::decide falls
/// back to its pre-existing default.
pub fn detect_class() -> Option<String> {
    // The D-Bus / X11 calls below have no timeout of their own, and this runs
    // on the single inject worker — a wedged Shell or X server must not stall
    // every paste. Bound the lookup, and skip it entirely while a previous
    // (hung) lookup is still outstanding.
    if LOOKUP_IN_FLIGHT.swap(true, Ordering::AcqRel) {
        return None;
    }
    let (tx, rx) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("emobie-focus-lookup".into())
        .spawn(move || {
            let class = detect_class_blocking();
            LOOKUP_IN_FLIGHT.store(false, Ordering::Release);
            let _ = tx.send(class);
        });
    if spawned.is_err() {
        LOOKUP_IN_FLIGHT.store(false, Ordering::Release);
        return None;
    }
    rx.recv_timeout(LOOKUP_TIMEOUT).ok().flatten()
}

fn detect_class_blocking() -> Option<String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        gnome::active_window_class().or_else(x11::active_window_class)
    } else {
        x11::active_window_class().or_else(gnome::active_window_class)
    }
}
