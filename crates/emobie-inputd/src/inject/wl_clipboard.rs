//! wl-copy / wl-paste helpers: on Wayland/KDE these are often more reliable
//! than arboard, and wl-paste runs as a separate client, so a read-back proves
//! the compositor is offering our text.

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::clipboard::{CLIPBOARD_READY_TIMEOUT, PASTE_SETTLE, POST_CLIPBOARD_SETTLE};

pub(super) fn wl_copy_available() -> bool {
    Command::new("wl-copy")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(super) fn set_via_wl_copy(body: &str) -> Result<(), String> {
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

/// Cap for reading the user's clipboard before an expansion replaces it.
const ORIGINAL_READ_TIMEOUT: Duration = Duration::from_millis(300);
const ORIGINAL_MAX_BYTES: u64 = 1024 * 1024;

/// Current clipboard text via wl-paste, or `None` when it is empty, not text,
/// or its owner does not answer in time.
pub(super) fn read_via_wl_paste() -> Option<String> {
    use std::io::Read;
    let mut child = Command::new("wl-paste")
        .args(["-n", "--type", "text"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = (&mut stdout).take(ORIGINAL_MAX_BYTES).read_to_end(&mut buf);
        buf
    });
    let deadline = Instant::now() + ORIGINAL_READ_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(PASTE_SETTLE),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let buf = reader.join().ok()?;
    if !status?.success() {
        return None;
    }
    String::from_utf8(buf).ok()
}
