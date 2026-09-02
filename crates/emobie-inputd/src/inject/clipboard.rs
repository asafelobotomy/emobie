use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// Poll interval while waiting for the compositor to advertise clipboard text.
const PASTE_SETTLE: Duration = Duration::from_millis(25);
/// Cold Wayland/KDE clipboards often need hundreds of ms before Ctrl+V sees our offer.
const CLIPBOARD_READY_TIMEOUT: Duration = Duration::from_millis(500);
/// Must outlive slow first-paste reads; restoring early pastes empty/old text.
/// (UI auto-paste uses 500ms; expand needs more headroom after idle.)
const CLIPBOARD_RESTORE_DELAY: Duration = Duration::from_millis(900);
/// Short ASCII fallback typing only (paste is the primary path).
const KEY_TYPE_MAX_CHARS: usize = 16;

/// Clipboard restore generation — only the latest expand restores the original.
static CLIPBOARD_EPOCH: AtomicU64 = AtomicU64::new(0);
static CLIPBOARD_ORIGINAL: Mutex<Option<String>> = Mutex::new(None);

/// True when fallback key-typing is unsafe (newline/tab/unicode/long).
pub(super) fn prefers_literal_insert(body: &str) -> bool {
    let mut chars = 0usize;
    for c in body.chars() {
        if c == '\n' || c == '\r' || c == '\t' || !c.is_ascii() {
            return true;
        }
        chars += 1;
        if chars > KEY_TYPE_MAX_CHARS {
            return true;
        }
    }
    false
}

/// Restore the pre-burst clipboard only if this expand is still the latest.
pub(super) fn schedule_clipboard_restore(expected: String, epoch: u64) {
    thread::spawn(move || {
        thread::sleep(CLIPBOARD_RESTORE_DELAY);
        if CLIPBOARD_EPOCH.load(Ordering::Acquire) != epoch {
            return;
        }
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let current = clipboard.get_text().unwrap_or_default();
            if current == expected {
                // Only restore when we captured a prior value. Clearing to "" on
                // a failed read races a still-in-flight paste and pastes nothing.
                let previous = CLIPBOARD_ORIGINAL.lock().ok().and_then(|mut g| g.take());
                if let Some(previous) = previous {
                    let _ = clipboard.set_text(previous);
                }
            } else if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
                // User copied something else — drop our claim on the original.
                *guard = None;
            }
        }
    });
}

pub(super) fn set_clipboard_text(body: &str) -> Result<u64, String> {
    let epoch = CLIPBOARD_EPOCH.fetch_add(1, Ordering::AcqRel) + 1;
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let current = clipboard.get_text().ok();
    if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
        // First expand in a burst captures the user's clipboard; later expands
        // in the same burst keep that original so restore does not chain.
        // Leave None when get_text failed so restore will not clear mid-paste.
        if guard.is_none() {
            if let Some(cur) = current {
                *guard = Some(cur);
            }
        }
    }

    // Offer text and wait until a read-back matches. Cold Wayland clipboards
    // often accept set_text before Ctrl+V can see the offer — first expand then
    // erases the trigger and pastes nothing; the second try works once warm.
    let deadline = Instant::now() + CLIPBOARD_READY_TIMEOUT;
    loop {
        match clipboard.set_text(body) {
            Ok(()) => {}
            Err(err) => {
                let set_err = err.to_string();
                match arboard::Clipboard::new() {
                    Ok(fresh) => clipboard = fresh,
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
        match clipboard.get_text() {
            Ok(got) if got == body => break,
            Ok(_) | Err(_) => {
                if Instant::now() >= deadline {
                    // Some compositors never echo get_text but still serve paste.
                    thread::sleep(PASTE_SETTLE);
                    break;
                }
                thread::sleep(PASTE_SETTLE);
            }
        }
    }
    Ok(epoch)
}

pub(super) fn restore_clipboard_now() {
    if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
        if let Some(prev) = guard.take() {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(prev);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::prefers_literal_insert;

    #[test]
    fn paragraphs_prefer_literal_insert() {
        assert!(prefers_literal_insert("line1\nline2"));
        assert!(prefers_literal_insert("line1\r\nline2"));
        assert!(prefers_literal_insert("a\tb"));
    }

    #[test]
    fn short_ascii_can_use_keys() {
        assert!(!prefers_literal_insert("hiya"));
        assert!(!prefers_literal_insert("hello world"));
    }

    #[test]
    fn long_and_unicode_prefer_literal() {
        assert!(prefers_literal_insert("😀"));
        assert!(prefers_literal_insert(&"a".repeat(17)));
        assert!(!prefers_literal_insert(&"a".repeat(16)));
    }
}
