mod clipboard;
mod enigo;
mod uinput;

use enigo::{
    ctrl_v_enigo, expand_with_enigo, new_enigo, retype_trigger_enigo, warm_up_enigo, ENIGO_MAX_IDLE,
    POST_PASTE_DELAY,
};
use uinput::{expand_with_uinput, retype_trigger_uinput};

use ::enigo::Enigo;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::listen;
use crate::session_env;
use crate::uinput_kbd::UInputKeyboard;

/// Ignore synthetic keys after inject finishes (Wayland can deliver late).
const SUPPRESS_GRACE: Duration = Duration::from_millis(150);
const INJECT_CACHE_TTL: Duration = Duration::from_secs(2);

/// Expand jobs queued or in-flight (listen buffer should ignore keys).
static LISTEN_SUPPRESS_JOBS: AtomicUsize = AtomicUsize::new(0);
static EXPAND_ENABLED: AtomicBool = AtomicBool::new(true);
/// Epoch millis until which listeners should keep suppressing after a job ends.
static SUPPRESS_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
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
                        expand_with_uinput(kbd, erase, &expansion, &trigger, trigger_committed)
                    }))
                } else {
                    let backend = enigo.as_mut().expect("enigo ensured");
                    catch_unwind(AssertUnwindSafe(|| {
                        expand_with_enigo(backend, erase, &expansion, &trigger, trigger_committed)
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
