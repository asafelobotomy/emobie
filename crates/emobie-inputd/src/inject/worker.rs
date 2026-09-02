//! Dedicated inject worker thread (uinput / Enigo).

use super::clipboard::last_backend;
use super::enigo::{
    ctrl_v_enigo, expand_with_enigo, new_enigo, retype_trigger_enigo, warm_up_enigo, ENIGO_MAX_IDLE,
    POST_PASTE_DELAY,
};
use super::uinput::{expand_with_uinput, retype_trigger_uinput};
use super::{finish_listen_suppress, now_ms, EXPAND_ENABLED, SUPPRESS_STARTED_MS};

use ::enigo::Enigo;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
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
    },
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
                // Refresh watchdog per job so a slow-but-healthy queue does not
                // trip force-open from the first job's start time.
                SUPPRESS_STARTED_MS.store(now_ms(), Ordering::Release);
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
