//! Dedicated inject worker thread (uinput / Enigo).

use super::clipboard::last_backend;
use super::enigo::{
    expand_with_enigo, new_enigo, paste_chord_enigo, retype_trigger_enigo,
    toggle_above_gnome_enigo, warm_up_enigo, ENIGO_MAX_IDLE, POST_PASTE_DELAY,
};
use super::uinput::{expand_with_uinput, retype_trigger_uinput, UINPUT_MAX_IDLE};
use super::{finish_listen_suppress, now_ms, EXPAND_ENABLED, REOPEN_UINPUT, SUPPRESS_STARTED_MS};

use ::enigo::Enigo;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

use crate::listen;
use crate::uinput_kbd::UInputKeyboard;

pub(super) enum InjectJob {
    Expand {
        erase: usize,
        expansion: String,
        trigger: String,
        trigger_committed: bool,
    },
    Paste {
        reply: mpsc::SyncSender<Result<(), String>>,
        /// Set by the caller when it gave up waiting; the job must not run late.
        cancel: Arc<AtomicBool>,
    },
    PinToggle {
        reply: mpsc::SyncSender<Result<(), String>>,
        cancel: Arc<AtomicBool>,
    },
}

impl InjectJob {
    fn is_cancelled(&self) -> bool {
        match self {
            InjectJob::Paste { cancel, .. } | InjectJob::PinToggle { cancel, .. } => {
                cancel.load(Ordering::Acquire)
            }
            InjectJob::Expand { .. } => false,
        }
    }
}

fn recover_after_expand_failure(
    uinput: &mut Option<UInputKeyboard>,
    enigo: &mut Option<Enigo>,
    trigger: &str,
    last_inject: &mut Instant,
    err: &str,
) {
    // Keep suppress through recovery so physical keys cannot interleave.
    if uinput.is_some() {
        *uinput = UInputKeyboard::open().ok();
        if let Some(kbd) = uinput.as_mut() {
            retype_trigger_uinput(kbd, trigger);
            *last_inject = Instant::now();
        }
    } else {
        *enigo = None;
        if let Ok(mut backend) = new_enigo() {
            retype_trigger_enigo(&mut backend, trigger);
            *enigo = Some(backend);
            *last_inject = Instant::now();
        }
    }
    listen::restore_to_buffer(trigger);
    eprintln!("expand failed: {err}");
    finish_listen_suppress();
}

/// Mirror `recover_after_expand_failure`'s reopen — a failed uinput write must
/// not permanently strand the backend on Enigo (which docs note cannot reach
/// native Wayland apps on Plasma). Reopening is the same recovery the Expand
/// path already relies on.
fn recover_backend_after_paste_failure(uinput: &mut Option<UInputKeyboard>, enigo: &mut Option<Enigo>) {
    if uinput.is_some() {
        *uinput = UInputKeyboard::open().ok();
    } else {
        *enigo = None;
    }
}

pub(super) fn inject_worker_loop(rx: mpsc::Receiver<InjectJob>) {
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
        // The requester timed out — a late Ctrl+V / pin chord would land in
        // whatever window is focused by now.
        if job.is_cancelled() {
            finish_listen_suppress();
            continue;
        }
        let resumed = REOPEN_UINPUT.swap(false, Ordering::AcqRel);
        if uinput.is_some() && (resumed || last_inject.elapsed() >= UINPUT_MAX_IDLE) {
            let refreshed = UInputKeyboard::open().ok();
            if refreshed.is_none() {
                eprintln!("emobie-inputd: uinput idle-refresh failed; Enigo fallback");
            }
            uinput = refreshed;
            last_inject = Instant::now();
        }
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
                            InjectJob::Paste { reply, .. } | InjectJob::PinToggle { reply, .. } => {
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
                // Refresh watchdog per job so a slow-but-healthy queue does not
                // trip force-open from the first job's start time.
                SUPPRESS_STARTED_MS.store(now_ms(), Ordering::Release);
                let excluded = crate::guard::has_excluded_apps()
                    && crate::guard::app_excluded(
                        crate::focused_window::detect_class().as_deref(),
                    );
                if excluded {
                    eprintln!("emobie-inputd: expand skipped (focused app is excluded)");
                }
                if !EXPAND_ENABLED.load(Ordering::Relaxed) || excluded {
                    // Nothing was erased: the trigger stays as typed.
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

                match expand_result {
                    Ok(Ok(())) => {
                        last_inject = Instant::now();
                        let backend = last_backend().unwrap_or("unknown");
                        eprintln!(
                            "emobie-inputd: expand ok backend={backend} ({} char trigger)",
                            trigger.chars().count()
                        );
                        finish_listen_suppress();
                    }
                    Ok(Err(err)) => {
                        eprintln!(
                            "emobie-inputd: expand fail backend={}",
                            last_backend().unwrap_or("unknown")
                        );
                        recover_after_expand_failure(
                            &mut uinput,
                            &mut enigo,
                            &trigger,
                            &mut last_inject,
                            &err,
                        );
                    }
                    Err(_) => {
                        eprintln!("emobie-inputd: expand fail backend=panic");
                        recover_after_expand_failure(
                            &mut uinput,
                            &mut enigo,
                            &trigger,
                            &mut last_inject,
                            "input injection backend panicked",
                        );
                    }
                }
            }
            InjectJob::Paste { reply, cancel } => {
                // Best-effort: falls back to the default Ctrl+V chord when
                // the focused app can't be identified (see paste_chord.rs).
                let chord = crate::paste_chord::decide(
                    crate::focused_window::detect_class().as_deref(),
                );
                if cancel.load(Ordering::Acquire) {
                    finish_listen_suppress();
                    continue;
                }
                let paste_result = if let Some(kbd) = uinput.as_mut() {
                    catch_unwind(AssertUnwindSafe(|| {
                        let result = kbd.paste_chord(chord);
                        if result.is_ok() {
                            thread::sleep(POST_PASTE_DELAY);
                        }
                        result
                    }))
                } else {
                    let backend = enigo.as_mut().expect("enigo ensured");
                    catch_unwind(AssertUnwindSafe(|| {
                        let result = paste_chord_enigo(backend, chord);
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
                        recover_backend_after_paste_failure(&mut uinput, &mut enigo);
                        Err(err)
                    }
                    Err(_) => {
                        recover_backend_after_paste_failure(&mut uinput, &mut enigo);
                        Err("input injection backend panicked".to_string())
                    }
                };
                let _ = reply.send(result);
                finish_listen_suppress();
            }
            InjectJob::PinToggle { reply, .. } => {
                let toggle_result = if let Some(kbd) = uinput.as_mut() {
                    catch_unwind(AssertUnwindSafe(|| kbd.toggle_above_gnome()))
                } else {
                    let backend = enigo.as_mut().expect("enigo ensured");
                    catch_unwind(AssertUnwindSafe(|| toggle_above_gnome_enigo(backend)))
                };
                let result = match toggle_result {
                    Ok(Ok(())) => {
                        last_inject = Instant::now();
                        Ok(())
                    }
                    Ok(Err(err)) => {
                        recover_backend_after_paste_failure(&mut uinput, &mut enigo);
                        Err(err)
                    }
                    Err(_) => {
                        recover_backend_after_paste_failure(&mut uinput, &mut enigo);
                        Err("input injection backend panicked".to_string())
                    }
                };
                let _ = reply.send(result);
                finish_listen_suppress();
            }
        }
    }
}
