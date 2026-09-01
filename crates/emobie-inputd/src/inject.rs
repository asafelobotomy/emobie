use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::session_env;

/// Only used when expand fires before the completing key is released (overlap).
const PRE_ERASE_DELAY: Duration = Duration::from_millis(12);
/// Brief settle so the focused app applies backspaces before insert.
const POST_ERASE_DELAY: Duration = Duration::from_millis(8);
const KEY_GAP: Duration = Duration::from_millis(1);
const PASTE_SETTLE: Duration = Duration::from_millis(10);
const CLIPBOARD_RESTORE_DELAY: Duration = Duration::from_millis(350);
/// Ignore synthetic keys for a short window after inject keystrokes finish.
const SUPPRESS_GRACE: Duration = Duration::from_millis(80);
/// Short ASCII fallback typing only (paste is the primary path).
const KEY_TYPE_MAX_CHARS: usize = 16;

/// Expand jobs queued or in-flight (listen buffer should ignore keys).
static LISTEN_SUPPRESS_JOBS: AtomicUsize = AtomicUsize::new(0);
static EXPAND_ENABLED: AtomicBool = AtomicBool::new(true);
/// Epoch millis until which listeners should keep suppressing after a job ends.
static SUPPRESS_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
static INJECT_TX: OnceLock<SyncSender<InjectJob>> = OnceLock::new();

enum InjectJob {
    Expand {
        erase: usize,
        expansion: String,
        trigger: String,
        trigger_committed: bool,
    },
    Paste {
        reply: SyncSender<Result<(), String>>,
    },
}

/// True while expand jobs are queued/in-flight or within the post-inject grace window.
pub fn set_expand_enabled(enabled: bool) {
    EXPAND_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn should_suppress_keys() -> bool {
    if LISTEN_SUPPRESS_JOBS.load(Ordering::Acquire) > 0 {
        return true;
    }
    let until = SUPPRESS_UNTIL_MS.load(Ordering::Acquire);
    now_ms() < until
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn arm_suppress_grace() {
    let until = now_ms().saturating_add(SUPPRESS_GRACE.as_millis() as u64);
    SUPPRESS_UNTIL_MS.store(until, Ordering::Release);
}

fn finish_listen_suppress() {
    LISTEN_SUPPRESS_JOBS.fetch_sub(1, Ordering::AcqRel);
    arm_suppress_grace();
}

fn compositor_available() -> bool {
    session_env::compositor_likely_available()
}

fn can_write_uinput() -> bool {
    for path in ["/dev/uinput", "/dev/input/uinput"] {
        if std::path::Path::new(path).exists()
            && std::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .is_ok()
        {
            return true;
        }
    }
    false
}

/// True when paste/inject is plausible in this session (compositor and/or uinput).
pub fn can_inject() -> bool {
    compositor_available() || can_write_uinput()
}

fn new_enigo() -> Result<Enigo, String> {
    catch_unwind(AssertUnwindSafe(|| {
        let mut settings = Settings::default();
        // Re-detect every time: systemd often starts us before Wayland exists.
        // Read-only — do not call ensure_session_env from worker threads.
        settings.wayland_display = session_env::wayland_display_for_enigo();
        Enigo::new(&settings)
    }))
    .map_err(|_| "input injection backend panicked".to_string())?
    .map_err(|e| e.to_string())
}

fn click_key(enigo: &mut Enigo, key: Key) -> Result<(), String> {
    enigo
        .key(key, Direction::Click)
        .map_err(|e| e.to_string())?;
    thread::sleep(KEY_GAP);
    Ok(())
}

fn ctrl_v(enigo: &mut Enigo) -> Result<(), String> {
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    let typed = (|| -> Result<(), String> {
        thread::sleep(KEY_GAP);
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| e.to_string())?;
        thread::sleep(KEY_GAP);
        Ok(())
    })();
    // Always release Control — a stuck modifier poisons the session.
    let released = enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string());
    typed.and(released)
}

fn erase_chars(enigo: &mut Enigo, count: usize) -> Result<(), String> {
    for _ in 0..count {
        click_key(enigo, Key::Backspace)?;
    }
    if count > 0 {
        thread::sleep(POST_ERASE_DELAY);
    }
    Ok(())
}

/// True when fallback key-typing is unsafe (newline/tab/unicode/long).
fn prefers_literal_insert(body: &str) -> bool {
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

/// Restore clipboard after inject finishes — must not block listen suppress.
fn schedule_clipboard_restore(previous: String, expected: String) {
    thread::spawn(move || {
        thread::sleep(CLIPBOARD_RESTORE_DELAY);
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let current = clipboard.get_text().unwrap_or_default();
            if current == expected {
                let _ = clipboard.set_text(previous);
            }
        }
    });
}

fn set_clipboard_text(body: &str) -> Result<Option<String>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(body)
        .map_err(|e| e.to_string())?;
    thread::sleep(PASTE_SETTLE);
    Ok(previous)
}

fn paste_with(enigo: &mut Enigo, body: &str) -> Result<(), String> {
    let previous = set_clipboard_text(body)?;
    ctrl_v(enigo)?;
    if let Some(prev) = previous {
        schedule_clipboard_restore(prev, body.to_string());
    }
    Ok(())
}

fn retype_trigger(enigo: &mut Enigo, trigger: &str) {
    if trigger.is_empty() {
        return;
    }
    if prefers_literal_insert(trigger) {
        let _ = paste_with(enigo, trigger);
    } else {
        let _ = enigo.text(trigger);
    }
}

fn expand_with(
    enigo: &mut Enigo,
    trigger_chars: usize,
    expansion: &str,
    trigger: &str,
    trigger_committed: bool,
) -> Result<(), String> {
    if !trigger_committed {
        thread::sleep(PRE_ERASE_DELAY);
    }

    // Clipboard first so erase failure can restore it; paste failure can retype trigger.
    if expansion.contains('\0') {
        return Err("expansion contains NUL".into());
    }
    let previous = if expansion.is_empty() {
        None
    } else {
        set_clipboard_text(expansion)?
    };

    if let Err(err) = erase_chars(enigo, trigger_chars) {
        if let Some(prev) = previous {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(prev);
            }
        }
        return Err(err);
    }

    if expansion.is_empty() {
        return Ok(());
    }

    match ctrl_v(enigo) {
        Ok(()) => {
            if let Some(prev) = previous {
                schedule_clipboard_restore(prev, expansion.to_string());
            }
            Ok(())
        }
        Err(paste_err) => {
            retype_trigger(enigo, trigger);
            if let Some(prev) = previous {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(prev);
                }
            }
            if !prefers_literal_insert(expansion) && enigo.text(expansion).is_ok() {
                return Ok(());
            }
            Err(paste_err)
        }
    }
}

fn inject_worker_loop(rx: mpsc::Receiver<InjectJob>) {
    let mut enigo: Option<Enigo> = None;
    while let Ok(job) = rx.recv() {
        if enigo.is_none() {
            match new_enigo() {
                Ok(backend) => enigo = Some(backend),
                Err(err) => {
                    match job {
                        InjectJob::Expand { .. } => {
                            finish_listen_suppress();
                            eprintln!("expand failed: {err}");
                        }
                        InjectJob::Paste { reply } => {
                            let _ = reply.send(Err(err.clone()));
                            finish_listen_suppress();
                        }
                    }
                    continue;
                }
            }
        }

        match job {
            InjectJob::Expand {
                erase,
                expansion,
                trigger,
                trigger_committed,
            } => {
                if !EXPAND_ENABLED.load(Ordering::Relaxed) {
                    finish_listen_suppress();
                    continue;
                }
                let expand_result = {
                    let backend = enigo.as_mut().expect("enigo just ensured");
                    catch_unwind(AssertUnwindSafe(|| {
                        expand_with(
                            backend,
                            erase,
                            &expansion,
                            &trigger,
                            trigger_committed,
                        )
                    }))
                };

                // Release listen suppress before any delayed clipboard work returns.
                finish_listen_suppress();
                match expand_result {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => {
                        enigo = None;
                        eprintln!("expand failed: {err}");
                    }
                    Err(_) => {
                        enigo = None;
                        eprintln!("expand failed: input injection backend panicked");
                    }
                }
            }
            InjectJob::Paste { reply } => {
                let paste_result = {
                    let backend = enigo.as_mut().expect("enigo just ensured");
                    catch_unwind(AssertUnwindSafe(|| ctrl_v(backend)))
                };
                let result = match paste_result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(err)) => {
                        enigo = None;
                        Err(err)
                    }
                    Err(_) => {
                        enigo = None;
                        Err("input injection backend panicked".to_string())
                    }
                };
                let _ = reply.send(result);
                finish_listen_suppress();
            }
        }
    }
}

fn inject_sender() -> SyncSender<InjectJob> {
    INJECT_TX
        .get_or_init(|| {
            let (tx, rx) = mpsc::sync_channel(32);
            thread::spawn(move || inject_worker_loop(rx));
            tx
        })
        .clone()
}

/// Queue a trigger replacement on the dedicated inject worker (serialized, reused Enigo).
/// Never blocks the listen thread — drops the job if the queue is full.
pub fn expand_trigger(
    trigger_chars: usize,
    expansion: &str,
    trigger: &str,
    trigger_committed: bool,
) -> Result<(), String> {
    LISTEN_SUPPRESS_JOBS.fetch_add(1, Ordering::AcqRel);
    match inject_sender().try_send(InjectJob::Expand {
        erase: trigger_chars,
        expansion: expansion.to_string(),
        trigger: trigger.to_string(),
        trigger_committed,
    }) {
        Ok(()) => Ok(()),
        Err(_) => {
            LISTEN_SUPPRESS_JOBS.fetch_sub(1, Ordering::AcqRel);
            Err("expand queue full — dropping job".to_string())
        }
    }
}

/// Queue Ctrl+V on the inject worker and wait for completion (serialized with expands).
pub fn inject_ctrl_v() -> Result<(), String> {
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    // Suppress listen briefly so synthetic Ctrl+V does not pollute the match buffer.
    LISTEN_SUPPRESS_JOBS.fetch_add(1, Ordering::AcqRel);
    match inject_sender().try_send(InjectJob::Paste { reply: reply_tx }) {
        Ok(()) => {}
        Err(_) => {
            finish_listen_suppress();
            return Err("inject queue full".to_string());
        }
    }
    match reply_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(result) => result,
        Err(_) => {
            // Worker still holds suppress until paste finishes — avoids surprise paste.
            Err("inject paste timed out".to_string())
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
