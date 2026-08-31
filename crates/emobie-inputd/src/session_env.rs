//! Ensure compositor env vars are set when started from systemd --user.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

/// systemd user units often start before the session exports WAYLAND_DISPLAY.
/// Detect the Wayland socket under XDG_RUNTIME_DIR so enigo can inject.
/// Does not set DISPLAY — forcing :0 on Wayland sessions can duplicate keystrokes via XWayland.
///
/// Call only from `main` before spawning worker threads — `set_var` is not thread-safe.
pub fn ensure_session_env() {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        if let Some(name) = detect_wayland_display() {
            // SAFETY: only called from main before listen/inject/accept threads start.
            unsafe {
                std::env::set_var("WAYLAND_DISPLAY", name);
            }
        }
    }
}

/// Block until a compositor socket appears (or timeout).
/// Read-only — safe to call from a background thread after workers have started.
/// Prefer running after the Unix socket is bound so clients can Status while we wait.
pub fn wait_for_compositor(timeout: Duration) {
    let start = Instant::now();
    loop {
        if compositor_likely_available() {
            return;
        }
        if start.elapsed() >= timeout {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
}

/// Display name for Enigo (`Settings.wayland_display`), re-detected each call.
pub fn wayland_display_for_enigo() -> Option<String> {
    if let Ok(name) = std::env::var("WAYLAND_DISPLAY") {
        if !name.is_empty() {
            let runtime = std::env::var("XDG_RUNTIME_DIR").ok()?;
            if PathBuf::from(&runtime).join(&name).exists() {
                return Some(name);
            }
        }
    }
    detect_wayland_display().map(str::to_string)
}

/// Read-only probe (safe on worker threads).
pub fn compositor_likely_available() -> bool {
    if let Ok(name) = std::env::var("WAYLAND_DISPLAY") {
        if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
            if PathBuf::from(runtime).join(&name).exists() {
                return true;
            }
        }
    }
    if std::env::var_os("DISPLAY").is_some() {
        return true;
    }
    detect_wayland_display().is_some()
}

fn detect_wayland_display() -> Option<&'static str> {
    let runtime = std::env::var("XDG_RUNTIME_DIR").ok()?;
    for name in ["wayland-0", "wayland-1", "wayland-2"] {
        if PathBuf::from(&runtime).join(name).exists() {
            return Some(name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::ensure_session_env;
    use std::path::PathBuf;

    #[test]
    fn ensure_session_env_is_idempotent_when_wayland_set() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        ensure_session_env();
        assert_eq!(
            std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
            Some("wayland-0")
        );
        let _ = std::env::remove_var("WAYLAND_DISPLAY");
    }

    #[test]
    fn ensure_session_env_detects_runtime_wayland_socket() {
        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        let sock = PathBuf::from(&runtime).join("wayland-0");
        if !sock.exists() {
            return;
        }
        let _ = std::env::remove_var("WAYLAND_DISPLAY");
        ensure_session_env();
        assert_eq!(
            std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
            Some("wayland-0")
        );
    }
}
