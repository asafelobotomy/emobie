use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// Poll interval while waiting for the compositor to advertise clipboard text.
const PASTE_SETTLE: Duration = Duration::from_millis(25);
/// Cold Wayland/KDE clipboards often need hundreds of ms before Ctrl+V sees our offer.
const CLIPBOARD_READY_TIMEOUT: Duration = Duration::from_millis(600);
/// Extra beat after read-back matches — KDE can echo get_text before paste works.
const POST_CLIPBOARD_SETTLE: Duration = Duration::from_millis(40);
/// arboard's native Wayland path only proves *we* can read our own offer back —
/// unlike wl-copy (verified externally via a separate wl-paste process), it gives
/// no signal that the compositor has broadcast the new selection to the focused
/// client yet. Ctrl+V fired inside that gap pastes nothing (silent empty paste).
/// Give the compositor extra time to propagate before releasing the inject worker.
const ARBOARD_WAYLAND_SETTLE: Duration = Duration::from_millis(220);
/// Hard cap for any arboard / Wayland clipboard round-trip.
const CLIPBOARD_OP_TIMEOUT: Duration = Duration::from_millis(1200);
/// Must outlive slow first-paste reads; restoring early pastes empty/old text.
const CLIPBOARD_RESTORE_DELAY: Duration = Duration::from_millis(900);

/// Clipboard restore generation — only the latest expand restores the original.
static CLIPBOARD_EPOCH: AtomicU64 = AtomicU64::new(0);
static CLIPBOARD_ORIGINAL: Mutex<Option<String>> = Mutex::new(None);
static CLIPBOARD_OP_SEQ: AtomicU64 = AtomicU64::new(0);
/// Default off — restore races are a top Expand failure mode on Plasma.
static RESTORE_CLIPBOARD: AtomicBool = AtomicBool::new(false);
static LAST_BACKEND: Mutex<Option<&'static str>> = Mutex::new(None);

pub use super::keys_type::prefers_literal_insert;

pub fn set_restore_clipboard(enabled: bool) {
    RESTORE_CLIPBOARD.store(enabled, Ordering::Relaxed);
}

pub fn restore_clipboard_enabled() -> bool {
    RESTORE_CLIPBOARD.load(Ordering::Relaxed)
}

pub fn last_backend() -> Option<&'static str> {
    LAST_BACKEND.lock().ok().and_then(|g| *g)
}

pub(super) fn note_backend(name: &'static str) {
    if let Ok(mut g) = LAST_BACKEND.lock() {
        *g = Some(name);
    }
}

fn op_alive(seq: u64) -> bool {
    CLIPBOARD_OP_SEQ.load(Ordering::Acquire) == seq
}

fn clipboard_timeout<T: Send + 'static>(
    op: &'static str,
    f: impl FnOnce(u64) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let seq = CLIPBOARD_OP_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f(seq));
    });
    match rx.recv_timeout(CLIPBOARD_OP_TIMEOUT) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            let _ = CLIPBOARD_OP_SEQ.compare_exchange(
                seq,
                seq + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            Err(format!(
                "clipboard {op} timed out after {CLIPBOARD_OP_TIMEOUT:?}"
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(format!("clipboard {op} worker exited without a result"))
        }
    }
}

fn wl_copy_available() -> bool {
    Command::new("wl-copy")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn set_via_wl_copy(body: &str) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("wl-copy spawn: {e}"))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "wl-copy missing stdin".to_string())?;
        stdin
            .write_all(body.as_bytes())
            .map_err(|e| format!("wl-copy write: {e}"))?;
    }
    let status = child
        .wait()
        .map_err(|e| format!("wl-copy wait: {e}"))?;
    if !status.success() {
        return Err(format!("wl-copy exit {status}"));
    }
    // Verify paste sees our offer when wl-paste exists.
    if Command::new("wl-paste")
        .arg("-n")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        let deadline = Instant::now() + CLIPBOARD_READY_TIMEOUT;
        loop {
            let out = Command::new("wl-paste")
                .arg("-n")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output();
            if let Ok(out) = out {
                if out.stdout == body.as_bytes() {
                    thread::sleep(POST_CLIPBOARD_SETTLE);
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                thread::sleep(POST_CLIPBOARD_SETTLE);
                return Ok(());
            }
            thread::sleep(PASTE_SETTLE);
        }
    }
    thread::sleep(POST_CLIPBOARD_SETTLE);
    Ok(())
}

/// True when arboard picked its native Wayland data-control backend (same check
/// arboard itself uses to choose Wayland over X11).
fn arboard_on_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn post_clipboard_settle() {
    if arboard_on_wayland() {
        thread::sleep(ARBOARD_WAYLAND_SETTLE);
    } else {
        thread::sleep(POST_CLIPBOARD_SETTLE);
    }
}

fn offer_text_until_ready(
    clipboard: &mut arboard::Clipboard,
    body: &str,
    seq: u64,
) -> Result<(), String> {
    let deadline = Instant::now() + CLIPBOARD_READY_TIMEOUT;
    loop {
        if !op_alive(seq) {
            return Err("clipboard op cancelled".into());
        }
        match clipboard.set_text(body) {
            Ok(()) => {}
            Err(err) => {
                let set_err = err.to_string();
                match arboard::Clipboard::new() {
                    Ok(fresh) => *clipboard = fresh,
                    Err(err2) => return Err(format!("{set_err}; recreate: {err2}")),
                }
                if Instant::now() >= deadline {
                    return Err(set_err);
                }
                thread::sleep(PASTE_SETTLE);
                continue;
            }
        }
        thread::sleep(PASTE_SETTLE);
        if !op_alive(seq) {
            return Err("clipboard op cancelled".into());
        }
        match clipboard.get_text() {
            Ok(got) if got == body => {
                post_clipboard_settle();
                return Ok(());
            }
            Ok(_) | Err(_) => {
                if Instant::now() >= deadline {
                    post_clipboard_settle();
                    return Ok(());
                }
                thread::sleep(PASTE_SETTLE);
            }
        }
    }
}

pub(super) fn schedule_clipboard_restore(expected: String, epoch: u64) {
    if !RESTORE_CLIPBOARD.load(Ordering::Relaxed) {
        return;
    }
    thread::spawn(move || {
        thread::sleep(CLIPBOARD_RESTORE_DELAY);
        let _ = clipboard_timeout("restore", move |seq| {
            if CLIPBOARD_EPOCH.load(Ordering::Acquire) != epoch || !op_alive(seq) {
                return Ok(());
            }
            if set_via_wl_copy_previous_if_match(&expected, epoch, seq).is_ok() {
                return Ok(());
            }
            let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
            if !op_alive(seq) || CLIPBOARD_EPOCH.load(Ordering::Acquire) != epoch {
                return Ok(());
            }
            let current = clipboard.get_text().unwrap_or_default();
            if current != expected {
                if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
                    *guard = None;
                }
                return Ok(());
            }
            let previous = CLIPBOARD_ORIGINAL.lock().ok().and_then(|mut g| g.take());
            let Some(previous) = previous.filter(|s| !s.is_empty()) else {
                return Ok(());
            };
            if CLIPBOARD_EPOCH.load(Ordering::Acquire) != epoch || !op_alive(seq) {
                if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
                    if guard.is_none() {
                        *guard = Some(previous);
                    }
                }
                return Ok(());
            }
            let _ = clipboard.set_text(previous);
            Ok(())
        });
    });
}

fn set_via_wl_copy_previous_if_match(expected: &str, epoch: u64, seq: u64) -> Result<(), String> {
    if !wl_copy_available() {
        return Err("no wl-copy".into());
    }
    let out = Command::new("wl-paste")
        .arg("-n")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if out.stdout != expected.as_bytes() {
        return Err("clipboard changed".into());
    }
    if CLIPBOARD_EPOCH.load(Ordering::Acquire) != epoch || !op_alive(seq) {
        return Ok(());
    }
    let previous = CLIPBOARD_ORIGINAL.lock().ok().and_then(|mut g| g.take());
    let Some(previous) = previous.filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    set_via_wl_copy(&previous)
}

pub(super) fn set_clipboard_text(body: &str) -> Result<u64, String> {
    let body = body.to_string();
    clipboard_timeout("set", move |seq| {
        if !op_alive(seq) {
            return Err("clipboard op cancelled".into());
        }
        // Prefer wl-copy on Wayland/KDE — often more reliable than arboard.
        if wl_copy_available() {
            match set_via_wl_copy(&body) {
                Ok(()) => {
                    note_backend("wl-copy");
                    return Ok(CLIPBOARD_EPOCH.fetch_add(1, Ordering::AcqRel) + 1);
                }
                Err(err) => {
                    eprintln!("emobie-inputd: wl-copy failed ({err}); arboard fallback");
                }
            }
        }
        if !op_alive(seq) {
            return Err("clipboard op cancelled".into());
        }
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        let current = clipboard.get_text().ok();
        if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
            if guard.is_none() {
                if let Some(cur) = current.filter(|s| !s.is_empty()) {
                    *guard = Some(cur);
                }
            }
        }
        offer_text_until_ready(&mut clipboard, &body, seq)?;
        if !op_alive(seq) {
            return Err("clipboard op cancelled".into());
        }
        note_backend("arboard");
        Ok(CLIPBOARD_EPOCH.fetch_add(1, Ordering::AcqRel) + 1)
    })
}

pub(super) fn ensure_clipboard_text(body: &str) -> Result<(), String> {
    let body = body.to_string();
    clipboard_timeout("ensure", move |seq| {
        if wl_copy_available() {
            let out = Command::new("wl-paste")
                .arg("-n")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output();
            if let Ok(out) = out {
                if out.stdout == body.as_bytes() {
                    return Ok(());
                }
            }
            set_via_wl_copy(&body)?;
            note_backend("wl-copy");
            return Ok(());
        }
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        if !op_alive(seq) {
            return Err("clipboard op cancelled".into());
        }
        match clipboard.get_text() {
            Ok(got) if got == body => Ok(()),
            _ => {
                offer_text_until_ready(&mut clipboard, &body, seq)?;
                note_backend("arboard");
                Ok(())
            }
        }
    })
}

pub(super) fn restore_clipboard_now() {
    if !RESTORE_CLIPBOARD.load(Ordering::Relaxed) {
        if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
            *guard = None;
        }
        return;
    }
    let _ = clipboard_timeout("restore_now", |seq| {
        let previous = {
            let Ok(guard) = CLIPBOARD_ORIGINAL.lock() else {
                return Ok(());
            };
            guard.clone().filter(|s| !s.is_empty())
        };
        let Some(prev) = previous else {
            return Ok(());
        };
        if !op_alive(seq) {
            return Ok(());
        }
        if wl_copy_available() && set_via_wl_copy(&prev).is_ok() {
            if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
                *guard = None;
            }
            return Ok(());
        }
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return Ok(());
        };
        if clipboard.set_text(prev).is_ok() {
            if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
                *guard = None;
            }
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::prefers_literal_insert;

    #[test]
    fn paragraphs_prefer_literal_insert() {
        assert!(prefers_literal_insert("line1\nline2"));
        assert!(prefers_literal_insert("a\tb"));
    }

    #[test]
    fn short_ascii_can_use_keys() {
        assert!(!prefers_literal_insert("hiya"));
    }
}
