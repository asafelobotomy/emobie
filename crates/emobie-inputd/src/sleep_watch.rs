//! Recover in place after suspend/resume.
//!
//! Confirmed bug: evdev keyboard reads and the uinput virtual device can go
//! silently dead after a real suspend/resume (a device thread stayed parked for
//! hours post-resume, delivering zero events, with no error to trigger any
//! recovery). On logind's `PrepareForSleep(false)` we reopen every input
//! device, recreate the virtual keyboard before the next job and reload the
//! keymap — the same effect as restarting the helper, without the restart
//! (which systemd logged as a failure on every resume and which dropped
//! pastes for a couple of seconds).
//!
//! Event-driven: the thread blocks on the D-Bus signal stream.

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

/// Best-effort: without logind this thread exits quietly and the inject
/// worker's idle refresh remains the fallback.
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
        if !signal.args()?.start {
            eprintln!("emobie-inputd: resumed from suspend — reopening input devices");
            crate::listen::note_resume();
            crate::inject::note_resume();
            crate::keymap::watch::bump();
        }
    }
    Ok(())
}
