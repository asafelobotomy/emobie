mod clipboard;
mod ei;
mod enigo;
mod keys_type;
mod uinput;
mod worker;

use worker::{inject_worker_loop, InjectJob};

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::session_env;

/// Ignore synthetic keys after inject finishes (Wayland can deliver late).
const SUPPRESS_GRACE: Duration = Duration::from_millis(150);
/// If an inject job never finishes, force-open listen without zeroing the job
/// counter (zeroing would desync later `finish_listen_suppress` calls).
/// Must exceed worst-case set+ensure clipboard budgets on a healthy path.
const SUPPRESS_STUCK_MS: u64 = 4_000;
const INJECT_CACHE_TTL: Duration = Duration::from_secs(2);

/// Expand jobs queued or in-flight (listen buffer should ignore keys).
static LISTEN_SUPPRESS_JOBS: AtomicUsize = AtomicUsize::new(0);
pub(super) static EXPAND_ENABLED: AtomicBool = AtomicBool::new(true);
/// Epoch millis until which listeners should keep suppressing after a job ends.
static SUPPRESS_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
/// When suppress jobs went from 0 → N, refreshed when each job starts on the worker.
pub(super) static SUPPRESS_STARTED_MS: AtomicU64 = AtomicU64::new(0);
/// Stuck-job escape: listen despite jobs > 0 until the counter drains.
static SUPPRESS_FORCE_OPEN: AtomicBool = AtomicBool::new(false);
static INJECT_TX: OnceLock<SyncSender<InjectJob>> = OnceLock::new();
static INJECT_CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

/// True while expand jobs are queued/in-flight or within the post-inject grace window.
pub fn set_expand_enabled(enabled: bool) {
    EXPAND_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn set_restore_clipboard(enabled: bool) {
    clipboard::set_restore_clipboard(enabled);
}

pub fn restore_clipboard_enabled() -> bool {
    clipboard::restore_clipboard_enabled()
}

pub fn suppress_job_count() -> usize {
    LISTEN_SUPPRESS_JOBS.load(Ordering::Acquire)
}

pub fn last_inject_backend() -> Option<&'static str> {
    clipboard::last_backend()
}

pub fn should_suppress_keys() -> bool {
    let jobs = LISTEN_SUPPRESS_JOBS.load(Ordering::Acquire);
    if jobs > 0 {
        let started = SUPPRESS_STARTED_MS.load(Ordering::Acquire);
        let now = now_ms();
        if started > 0 && now.saturating_sub(started) >= SUPPRESS_STUCK_MS {
            if !SUPPRESS_FORCE_OPEN.swap(true, Ordering::AcqRel) {
                eprintln!(
                    "emobie-inputd: force-open listen (stuck suppress, {jobs} job(s) > {SUPPRESS_STUCK_MS}ms)"
                );
            }
            // Do not zero LISTEN_SUPPRESS_JOBS — that desyncs finish_listen_suppress.
            return false;
        }
        if SUPPRESS_FORCE_OPEN.load(Ordering::Acquire) {
            return false;
        }
        return true;
    }
    if SUPPRESS_FORCE_OPEN.load(Ordering::Acquire) {
        SUPPRESS_FORCE_OPEN.store(false, Ordering::Release);
    }
    let until = SUPPRESS_UNTIL_MS.load(Ordering::Acquire);
    now_ms() < until
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn arm_suppress_grace() {
    let until = now_ms().saturating_add(SUPPRESS_GRACE.as_millis() as u64);
    SUPPRESS_UNTIL_MS.store(until, Ordering::Release);
}

pub(super) fn finish_listen_suppress() {
    // Saturating decrement — never underflow if a path double-finishes.
    let mut prev = LISTEN_SUPPRESS_JOBS.load(Ordering::Acquire);
    loop {
        if prev == 0 {
            SUPPRESS_STARTED_MS.store(0, Ordering::Release);
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
                if prev == 1 {
                    SUPPRESS_STARTED_MS.store(0, Ordering::Release);
                }
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
    let prev_jobs = LISTEN_SUPPRESS_JOBS.fetch_add(1, Ordering::AcqRel);
    if prev_jobs == 0 {
        SUPPRESS_STARTED_MS.store(now_ms(), Ordering::Release);
    }
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
    let prev_jobs = LISTEN_SUPPRESS_JOBS.fetch_add(1, Ordering::AcqRel);
    if prev_jobs == 0 {
        SUPPRESS_STARTED_MS.store(now_ms(), Ordering::Release);
    }
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
