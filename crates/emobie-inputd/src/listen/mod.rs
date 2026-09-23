//! Trigger matching from one or more keyboard event devices.
//!
//! Key → char mapping uses libxkbcommon (session layout via XKB_DEFAULT_*).
//! Expansion runs on key *release* of the completing key so the focused app
//! has committed the trigger before we erase it.

mod devices;
mod keys;

pub use devices::can_listen;

use evdev::{Device, InputEventKind};
use keys::{expire_stale_pending, handle_key, trim_buffer, PendingExpand};
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use std::collections::HashSet;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::matcher::TriggerTrie;

use devices::list_input_paths;

/// Hotplug scan interval — avoid opening every event node more often than needed.
const HOTPLUG_INTERVAL: Duration = Duration::from_secs(5);
/// How long a device thread waits for input before re-checking `enabled`/`stop`.
const DEVICE_POLL_MS: u16 = 500;
/// Idle wait while expansion is disabled (no keyboard device is open).
const DISABLED_RECHECK: Duration = Duration::from_secs(1);

struct Shared {
    enabled: Arc<AtomicBool>,
    trie: Arc<Mutex<TriggerTrie>>,
    stop: Arc<AtomicBool>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();
static STARTED: AtomicBool = AtomicBool::new(false);

/// Register the state the listener needs. Does not start listening.
pub fn configure(enabled: Arc<AtomicBool>, trie: Arc<Mutex<TriggerTrie>>, stop: Arc<AtomicBool>) {
    let _ = SHARED.set(Shared {
        enabled,
        trie,
        stop,
    });
}

/// Start the keyboard listener (once). Called only when expansion is enabled,
/// so a daemon that is only used for paste never opens keyboard event nodes.
pub fn ensure_running() {
    let Some(shared) = SHARED.get() else {
        return;
    };
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    spawn_listener(
        shared.enabled.clone(),
        shared.trie.clone(),
        shared.stop.clone(),
    );
}

/// Wait until the device has input (or the timeout elapses). Returns true when
/// there is something to read or the fd reported an error/hangup.
fn wait_readable(device: &Device) -> bool {
    // SAFETY: `device` owns the fd and outlives this call.
    let fd = unsafe { BorrowedFd::borrow_raw(device.as_raw_fd()) };
    let mut fds = [PollFd::new(fd, PollFlags::POLLIN)];
    match poll(&mut fds, PollTimeout::from(DEVICE_POLL_MS)) {
        Ok(0) => false,
        Ok(_) => true,
        Err(_) => true,
    }
}

/// Bumped on resume: device threads close and the scan reopens them, since
/// an fd held across suspend can stay open yet never deliver events again.
static RESUME_EPOCH: AtomicU64 = AtomicU64::new(0);
/// Longer than `DEVICE_POLL_MS`, so device threads have exited (freeing their
/// slot) before the rescan, and udev has re-announced devices.
const RESUME_RESCAN_DELAY: Duration = Duration::from_millis(800);

pub fn note_resume() {
    RESUME_EPOCH.fetch_add(1, Ordering::AcqRel);
    if let Some(pending) = PENDING_HOLDER.get() {
        if let Ok(mut guard) = pending.lock() {
            *guard = None;
        }
    }
    if let Some(buffer) = BUFFER_HOLDER.get() {
        if let Ok(mut guard) = buffer.lock() {
            guard.clear();
        }
    }
}

/// Sleep for `interval`, returning early (after a short settle) on resume.
fn sleep_until_rescan(interval: Duration) {
    let epoch = RESUME_EPOCH.load(Ordering::Acquire);
    let deadline = Instant::now() + interval;
    while Instant::now() < deadline {
        if RESUME_EPOCH.load(Ordering::Acquire) != epoch {
            thread::sleep(RESUME_RESCAN_DELAY);
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

static PENDING_HOLDER: OnceLock<Arc<Mutex<Option<PendingExpand>>>> = OnceLock::new();
static BUFFER_HOLDER: OnceLock<Arc<Mutex<String>>> = OnceLock::new();
/// Serialize key handling across device threads that share one buffer/pending.
static KEY_HANDLER: Mutex<()> = Mutex::new(());

/// Drop any pending expand (e.g. when expansion is disabled), restoring the
/// trigger into the match buffer so it stays aligned with the focused app.
pub fn clear_pending() {
    let Some(pending) = PENDING_HOLDER.get() else {
        return;
    };
    let Some(buffer) = BUFFER_HOLDER.get() else {
        if let Ok(mut guard) = pending.lock() {
            *guard = None;
        }
        return;
    };
    let cancelled = {
        let Ok(mut guard) = pending.lock() else {
            return;
        };
        guard.take()
    };
    if let Some(p) = cancelled {
        if let Ok(mut guard) = buffer.lock() {
            guard.push_str(&p.trigger);
            trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
        }
    }
}

/// Push text back into the match buffer (e.g. when a queued expand is cancelled).
pub fn restore_to_buffer(text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(buffer) = BUFFER_HOLDER.get() {
        if let Ok(mut guard) = buffer.lock() {
            guard.push_str(text);
            trim_buffer(&mut guard, crate::state::MAX_TRIGGER_LEN);
        }
    }
}

fn spawn_device_thread(
    path: PathBuf,
    enabled: Arc<AtomicBool>,
    trie: Arc<Mutex<TriggerTrie>>,
    buffer: Arc<Mutex<String>>,
    pending: Arc<Mutex<Option<PendingExpand>>>,
    stop: Arc<AtomicBool>,
    alive: Arc<Mutex<HashSet<PathBuf>>>,
) {
    thread::spawn(move || {
        // Whatever happens below (including a panic, e.g. no xkb data on the
        // host), the path must leave `alive` so the hotplug scan can retry it.
        let cleanup_path = path.clone();
        let cleanup_alive = alive.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let keymap = crate::keymap::shared();
            let epoch = RESUME_EPOCH.load(Ordering::Acquire);
            let result = (|| -> Result<(), ()> {
                let mut device = Device::open(&path).map_err(|_| ())?;
                loop {
                    if stop.load(Ordering::Relaxed) || RESUME_EPOCH.load(Ordering::Acquire) != epoch {
                        return Ok(());
                    }
                    // Expansion switched off: close the device promptly instead of
                    // continuing to read the keyboard.
                    if !enabled.load(Ordering::Relaxed) {
                        if let Ok(mut guard) = buffer.lock() {
                            guard.clear();
                        }
                        return Ok(());
                    }
                    keymap.reload_periodically();
                    expire_stale_pending(&pending, &buffer);
                    if !wait_readable(&device) {
                        continue;
                    }
                    let events = match device.fetch_events() {
                        Ok(events) => events,
                        Err(_) => {
                            thread::sleep(Duration::from_millis(50));
                            return Err(());
                        }
                    };
                    keymap.reload_if_layout_changed();
                    for event in events {
                        if let InputEventKind::Key(key) = event.kind() {
                            let _gate = KEY_HANDLER.lock().unwrap_or_else(|e| e.into_inner());
                            handle_key(
                                key,
                                event.value(),
                                keymap,
                                &enabled,
                                &buffer,
                                &trie,
                                &pending,
                            );
                        }
                    }
                }
            })();
            let _ = result;
        }));
        let _ = cleanup_alive
            .lock()
            .map(|mut guard| guard.remove(&cleanup_path));
    });
}

fn spawn_listener(enabled: Arc<AtomicBool>, trie: Arc<Mutex<TriggerTrie>>, stop: Arc<AtomicBool>) {
    thread::spawn(move || {
        let buffer = Arc::new(Mutex::new(String::new()));
        let pending = Arc::new(Mutex::new(None));
        let _ = PENDING_HOLDER.set(pending.clone());
        let _ = BUFFER_HOLDER.set(buffer.clone());
        let alive = Arc::new(Mutex::new(HashSet::<PathBuf>::new()));
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if !enabled.load(Ordering::Relaxed) {
                thread::sleep(DISABLED_RECHECK);
                continue;
            }
            let paths = list_input_paths();
            if paths.is_empty() {
                sleep_until_rescan(HOTPLUG_INTERVAL);
                continue;
            }
            for path in paths {
                let already = alive.lock().map(|g| g.contains(&path)).unwrap_or(true);
                if already {
                    continue;
                }
                if let Ok(mut guard) = alive.lock() {
                    guard.insert(path.clone());
                }
                spawn_device_thread(
                    path,
                    enabled.clone(),
                    trie.clone(),
                    buffer.clone(),
                    pending.clone(),
                    stop.clone(),
                    alive.clone(),
                );
            }
            sleep_until_rescan(HOTPLUG_INTERVAL);
        }
    });
}
