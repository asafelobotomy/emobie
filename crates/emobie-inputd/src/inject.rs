use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::listen;
use crate::session_env;
use crate::uinput_kbd::UInputKeyboard;

/// Only used when expand fires before the completing key is released (overlap).
const PRE_ERASE_DELAY: Duration = Duration::from_millis(12);
/// Brief settle so the focused app applies backspaces before insert.
const POST_ERASE_DELAY: Duration = Duration::from_millis(8);
const KEY_GAP: Duration = Duration::from_millis(1);
/// Poll interval while waiting for the compositor to advertise clipboard text.
const PASTE_SETTLE: Duration = Duration::from_millis(25);
/// Cold Wayland/KDE clipboards often need hundreds of ms before Ctrl+V sees our offer.
const CLIPBOARD_READY_TIMEOUT: Duration = Duration::from_millis(500);
/// Must outlive slow first-paste reads; restoring early pastes empty/old text.
/// (UI auto-paste uses 500ms; expand needs more headroom after idle.)
const CLIPBOARD_RESTORE_DELAY: Duration = Duration::from_millis(900);
/// Let the focused app process Ctrl+V before we drop listen suppress.
const POST_PASTE_DELAY: Duration = Duration::from_millis(40);
/// Ignore synthetic keys after inject finishes (Wayland can deliver late).
const SUPPRESS_GRACE: Duration = Duration::from_millis(150);
/// Short ASCII fallback typing only (paste is the primary path).
const KEY_TYPE_MAX_CHARS: usize = 16;
const INJECT_CACHE_TTL: Duration = Duration::from_secs(2);
/// Recreate Enigo after idle — Wayland virtual-keyboard seats go stale.
const ENIGO_MAX_IDLE: Duration = Duration::from_secs(45);

/// Expand jobs queued or in-flight (listen buffer should ignore keys).
static LISTEN_SUPPRESS_JOBS: AtomicUsize = AtomicUsize::new(0);
static EXPAND_ENABLED: AtomicBool = AtomicBool::new(true);
/// Epoch millis until which listeners should keep suppressing after a job ends.
static SUPPRESS_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
/// Clipboard restore generation — only the latest expand restores the original.
static CLIPBOARD_EPOCH: AtomicU64 = AtomicU64::new(0);
static CLIPBOARD_ORIGINAL: Mutex<Option<String>> = Mutex::new(None);
static INJECT_TX: OnceLock<SyncSender<InjectJob>> = OnceLock::new();
static INJECT_CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

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
    // Saturating decrement — never underflow if a path double-finishes.
    let mut prev = LISTEN_SUPPRESS_JOBS.load(Ordering::Acquire);
    loop {
        if prev == 0 {
            arm_suppress_grace();
            return;
        }
        match LISTEN_SUPPRESS_JOBS.compare_exchange(
            prev,
            prev - 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                arm_suppress_grace();
                return;
            }
            Err(v) => prev = v,
        }
    }
}

fn compositor_available() -> bool {
    session_env::compositor_likely_available()
}

fn wayland_session() -> bool {
    session_env::wayland_display_for_enigo().is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
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

/// True when paste/inject is plausible in this session.
///
/// On Wayland, Plasma often lacks `zwp_virtual_keyboard`, so Enigo/XTest cannot
/// reach native clients (Cursor, etc.). Require writable `/dev/uinput` there.
/// On X11, compositor/Enigo remains sufficient (with uinput as a bonus).
pub fn can_inject() -> bool {
    if let Ok(cache) = INJECT_CACHE.lock() {
        if let Some((at, ok)) = *cache {
            if at.elapsed() < INJECT_CACHE_TTL {
                return ok;
            }
        }
    }
    let ok = if wayland_session() {
        can_write_uinput()
    } else {
        compositor_available() || can_write_uinput()
    };
    if let Ok(mut cache) = INJECT_CACHE.lock() {
        *cache = Some((Instant::now(), ok));
    }
    ok
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

/// Map paste chord keys into the virtual keymap and let the compositor adopt it.
/// Without this, the first Unicode('v') regenerates the keymap mid-chord and the
/// Click is dropped — trigger is erased, expansion never appears; retry works.
fn warm_up_enigo(enigo: &mut Enigo) {
    for c in ['v', 'V', 'c', 'C'] {
        let _ = enigo.key(Key::Unicode(c), Direction::Release);
    }
    let _ = enigo.key(Key::Control, Direction::Release);
    let _ = enigo.key(Key::Shift, Direction::Release);
    let _ = enigo.key(Key::Alt, Direction::Release);
    let _ = enigo.key(Key::Meta, Direction::Release);
    thread::sleep(Duration::from_millis(40));
}

fn click_key(enigo: &mut Enigo, key: Key) -> Result<(), String> {
    enigo
        .key(key, Direction::Click)
        .map_err(|e| e.to_string())?;
    thread::sleep(KEY_GAP);
    Ok(())
}

fn ensure_paste_key_mapped(enigo: &mut Enigo) {
    // Release-only maps the keysym + applies keymap without inserting a character.
    let _ = enigo.key(Key::Unicode('v'), Direction::Release);
    thread::sleep(Duration::from_millis(15));
}

fn ctrl_v_enigo(enigo: &mut Enigo) -> Result<(), String> {
    ensure_paste_key_mapped(enigo);
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
    let count = count.min(crate::state::MAX_TRIGGER_LEN);
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

/// Restore the pre-burst clipboard only if this expand is still the latest.
fn schedule_clipboard_restore(expected: String, epoch: u64) {
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

fn set_clipboard_text(body: &str) -> Result<u64, String> {
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

fn restore_clipboard_now() {
    if let Ok(mut guard) = CLIPBOARD_ORIGINAL.lock() {
        if let Some(prev) = guard.take() {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(prev);
            }
        }
    }
}

fn paste_with_uinput(kbd: &mut UInputKeyboard, body: &str) -> Result<(), String> {
    let epoch = set_clipboard_text(body)?;
    kbd.ctrl_v()?;
    thread::sleep(POST_PASTE_DELAY);
    schedule_clipboard_restore(body.to_string(), epoch);
    Ok(())
}

fn paste_with_enigo(enigo: &mut Enigo, body: &str) -> Result<(), String> {
    let epoch = set_clipboard_text(body)?;
    ctrl_v_enigo(enigo)?;
    thread::sleep(POST_PASTE_DELAY);
    schedule_clipboard_restore(body.to_string(), epoch);
    Ok(())
}

fn retype_trigger_uinput(kbd: &mut UInputKeyboard, trigger: &str) {
    if trigger.is_empty() {
        return;
    }
    let _ = paste_with_uinput(kbd, trigger);
}

fn retype_trigger_enigo(enigo: &mut Enigo, trigger: &str) {
    if trigger.is_empty() {
        return;
    }
    if prefers_literal_insert(trigger) {
        let _ = paste_with_enigo(enigo, trigger);
    } else {
        let _ = enigo.text(trigger);
    }
}

fn expand_with_uinput(
    kbd: &mut UInputKeyboard,
    trigger_chars: usize,
    expansion: &str,
    trigger: &str,
    trigger_committed: bool,
) -> Result<(), String> {
    if !trigger_committed {
        thread::sleep(PRE_ERASE_DELAY);
    }
    if expansion.contains('\0') {
        return Err("expansion contains NUL".into());
    }
    let epoch = if expansion.is_empty() {
        None
    } else {
        Some(set_clipboard_text(expansion)?)
    };

    let erase = trigger_chars.min(crate::state::MAX_TRIGGER_LEN);
    if let Err(err) = kbd.erase_chars(erase) {
        if epoch.is_some() {
            restore_clipboard_now();
        }
        return Err(err);
    }

    if expansion.is_empty() {
        return Ok(());
    }

    match kbd.ctrl_v() {
        Ok(()) => {
            thread::sleep(POST_PASTE_DELAY);
            if let Some(epoch) = epoch {
                schedule_clipboard_restore(expansion.to_string(), epoch);
            }
            Ok(())
        }
        Err(paste_err) => {
            retype_trigger_uinput(kbd, trigger);
            restore_clipboard_now();
            Err(paste_err)
        }
    }
}

fn expand_with_enigo(
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
    let epoch = if expansion.is_empty() {
        None
    } else {
        Some(set_clipboard_text(expansion)?)
    };

    if let Err(err) = erase_chars(enigo, trigger_chars) {
        if epoch.is_some() {
            restore_clipboard_now();
        }
        return Err(err);
    }

    if expansion.is_empty() {
        return Ok(());
    }

    match ctrl_v_enigo(enigo) {
        Ok(()) => {
            // Give the focused app time to handle paste before suppress ends.
            thread::sleep(POST_PASTE_DELAY);
            if let Some(epoch) = epoch {
                schedule_clipboard_restore(expansion.to_string(), epoch);
            }
            Ok(())
        }
        Err(paste_err) => {
            retype_trigger_enigo(enigo, trigger);
            restore_clipboard_now();
            if !prefers_literal_insert(expansion) && enigo.text(expansion).is_ok() {
                return Ok(());
            }
            Err(paste_err)
        }
    }
}

fn inject_worker_loop(rx: mpsc::Receiver<InjectJob>) {
    // Prefer uinput: reaches native Wayland (Cursor, Plasma apps). Enigo is
    // fallback when /dev/uinput is unavailable (no Grant / missing udev).
    let mut uinput = match UInputKeyboard::open() {
        Ok(kbd) => {
            eprintln!("emobie-inputd: inject via /dev/uinput");
            Some(kbd)
        }
        Err(err) => {
            eprintln!("emobie-inputd: uinput unavailable ({err}); Enigo fallback");
            None
        }
    };
    let mut enigo: Option<Enigo> = None;
    let mut last_inject = Instant::now();

    while let Ok(job) = rx.recv() {
        if uinput.is_none() {
            if enigo.is_some() && last_inject.elapsed() >= ENIGO_MAX_IDLE {
                enigo = None;
            }
            if enigo.is_none() {
                match new_enigo() {
                    Ok(mut backend) => {
                        warm_up_enigo(&mut backend);
                        enigo = Some(backend);
                    }
                    Err(err) => {
                        match job {
                            InjectJob::Expand { trigger, .. } => {
                                listen::restore_to_buffer(&trigger);
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
        }

        match job {
            InjectJob::Expand {
                erase,
                expansion,
                trigger,
                trigger_committed,
            } => {
                if !EXPAND_ENABLED.load(Ordering::Relaxed) {
                    listen::restore_to_buffer(&trigger);
                    finish_listen_suppress();
                    continue;
                }
                let expand_result = if let Some(kbd) = uinput.as_mut() {
                    catch_unwind(AssertUnwindSafe(|| {
                        expand_with_uinput(
                            kbd,
                            erase,
                            &expansion,
                            &trigger,
                            trigger_committed,
                        )
                    }))
                } else {
                    let backend = enigo.as_mut().expect("enigo ensured");
                    catch_unwind(AssertUnwindSafe(|| {
                        expand_with_enigo(
                            backend,
                            erase,
                            &expansion,
                            &trigger,
                            trigger_committed,
                        )
                    }))
                };

                finish_listen_suppress();
                match expand_result {
                    Ok(Ok(())) => {
                        last_inject = Instant::now();
                    }
                    Ok(Err(err)) => {
                        if uinput.is_some() {
                            // Recreate uinput device; best-effort retype via paste.
                            uinput = UInputKeyboard::open().ok();
                            if let Some(kbd) = uinput.as_mut() {
                                retype_trigger_uinput(kbd, &trigger);
                                last_inject = Instant::now();
                            }
                        } else {
                            enigo = None;
                            if let Ok(mut backend) = new_enigo() {
                                retype_trigger_enigo(&mut backend, &trigger);
                                enigo = Some(backend);
                                last_inject = Instant::now();
                            }
                        }
                        listen::restore_to_buffer(&trigger);
                        eprintln!("expand failed: {err}");
                    }
                    Err(_) => {
                        if uinput.is_some() {
                            uinput = UInputKeyboard::open().ok();
                            if let Some(kbd) = uinput.as_mut() {
                                retype_trigger_uinput(kbd, &trigger);
                                last_inject = Instant::now();
                            }
                        } else {
                            enigo = None;
                            if let Ok(mut backend) = new_enigo() {
                                retype_trigger_enigo(&mut backend, &trigger);
                                enigo = Some(backend);
                                last_inject = Instant::now();
                            }
                        }
                        listen::restore_to_buffer(&trigger);
                        eprintln!("expand failed: input injection backend panicked");
                    }
                }
            }
            InjectJob::Paste { reply } => {
                let paste_result = if let Some(kbd) = uinput.as_mut() {
                    catch_unwind(AssertUnwindSafe(|| {
                        let result = kbd.ctrl_v();
                        if result.is_ok() {
                            thread::sleep(POST_PASTE_DELAY);
                        }
                        result
                    }))
                } else {
                    let backend = enigo.as_mut().expect("enigo ensured");
                    catch_unwind(AssertUnwindSafe(|| {
                        let result = ctrl_v_enigo(backend);
                        if result.is_ok() {
                            thread::sleep(POST_PASTE_DELAY);
                        }
                        result
                    }))
                };
                let result = match paste_result {
                    Ok(Ok(())) => {
                        last_inject = Instant::now();
                        Ok(())
                    }
                    Ok(Err(err)) => {
                        if uinput.is_some() {
                            uinput = None;
                        } else {
                            enigo = None;
                        }
                        Err(err)
                    }
                    Err(_) => {
                        if uinput.is_some() {
                            uinput = None;
                        } else {
                            enigo = None;
                        }
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
    // Prefer the smaller of the caller erase count and the trigger string length,
    // then clamp to the match-size cap so we never schedule runaway Backspaces.
    let erase = trigger_chars
        .min(trigger.chars().count())
        .min(crate::state::MAX_TRIGGER_LEN);
    LISTEN_SUPPRESS_JOBS.fetch_add(1, Ordering::AcqRel);
    match inject_sender().try_send(InjectJob::Expand {
        erase,
        expansion: expansion.to_string(),
        trigger: trigger.to_string(),
        trigger_committed,
    }) {
        Ok(()) => Ok(()),
        Err(_) => {
            // No job ran — drop the suppress count without arming inject grace.
            let mut prev = LISTEN_SUPPRESS_JOBS.load(Ordering::Acquire);
            while prev > 0 {
                match LISTEN_SUPPRESS_JOBS.compare_exchange(
                    prev,
                    prev - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(v) => prev = v,
                }
            }
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
