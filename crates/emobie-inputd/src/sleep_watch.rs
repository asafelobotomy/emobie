//! Restart cleanly on resume from suspend — the fix for a confirmed bug:
//! evdev keyboard reads and the uinput virtual device can go silently dead
//! after a real suspend/resume cycle (observed directly: a device thread
//! stayed parked in `evdev_read` for hours post-resume, delivering zero
//! events, with no error to trigger any recovery path). `systemctl --user
//! restart emobie-inputd` is the confirmed fix — this listens for logind's
//! `PrepareForSleep` signal and performs that exact restart automatically,
//! right when it's needed, instead of on a blind timer or a polling loop.
//!
//! Event-driven, not polling: the thread blocks on the D-Bus connection
//! (like the existing ctrlc signal handler) and does nothing until the
//! system actually resumes. `Restart=on-failure` in the systemd unit brings
//! the process back up within `RestartSec=2`, re-opening every device,
//! the uinput handle, and the session environment from scratch.

use zbus::blocking::Connection;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

/// Best-effort: a missing logind (no systemd, minimal container, sandboxed
/// edge case) just means this thread exits quietly. The idle-refresh fixes
/// in the inject worker remain the fallback for staleness this cannot see.
pub fn spawn() {
    std::thread::spawn(|| {
        if let Err(err) = watch() {
            eprintln!("emobie-inputd: suspend/resume watch unavailable ({err}); skipping");
        }
    });
}

fn watch() -> zbus::Result<()> {
    let conn = Connection::system()?;
    let proxy = Login1ManagerProxyBlocking::new(&conn)?;
    for signal in proxy.receive_prepare_for_sleep()? {
        let args = signal.args()?;
        if !args.start {
            // Exiting is only a restart when a supervisor brings us back.
            // A detached helper (no systemd unit) would just die, so stay up.
            if std::env::var_os("INVOCATION_ID").is_none() {
                eprintln!(
                    "emobie-inputd: resumed from suspend but not supervised by systemd; \
                     keeping the current process"
                );
                continue;
            }
            eprintln!("emobie-inputd: resumed from suspend — restarting for a clean state");
            // Non-zero so systemd's Restart=on-failure brings it back up;
            // a plain exit(0) would not trigger a restart.
            std::process::exit(1);
        }
    }
    Ok(())
}
