//! Event-driven notice of layout switches, so the keymap follows the active
//! layout instead of waiting for the periodic reload.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

static GENERATION: AtomicU64 = AtomicU64::new(0);
static PLASMA_GROUP: AtomicU32 = AtomicU32::new(0);
static STARTED: AtomicBool = AtomicBool::new(false);

pub fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

pub fn plasma_group() -> u32 {
    PLASMA_GROUP.load(Ordering::Acquire)
}

/// Also used after resume, when the session layout may have changed.
pub fn bump() {
    GENERATION.fetch_add(1, Ordering::AcqRel);
}

/// Idempotent; best-effort (a missing gsettings / KWin just means we rely on
/// the periodic reload).
pub fn start() {
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    if super::sources::is_gnome() {
        std::thread::spawn(watch_gnome);
    }
    if super::sources::is_plasma() {
        std::thread::spawn(|| {
            if let Err(err) = watch_plasma() {
                eprintln!("emobie-inputd: Plasma layout watch unavailable ({err})");
            }
        });
    }
}

fn watch_gnome() {
    let child = Command::new("gsettings")
        .args(["monitor", "org.gnome.desktop.input-sources"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return;
    };
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.starts_with("sources:")
                || line.starts_with("mru-sources:")
                || line.starts_with("xkb-options:")
            {
                bump();
            }
        }
    }
    let _ = child.wait();
}

#[zbus::proxy(
    interface = "org.kde.KeyboardLayouts",
    default_service = "org.kde.keyboard",
    default_path = "/Layouts"
)]
trait KeyboardLayouts {
    #[zbus(name = "getLayout")]
    fn get_layout(&self) -> zbus::Result<u32>;

    #[zbus(signal, name = "layoutChanged")]
    fn layout_changed(&self, index: u32) -> zbus::Result<()>;

    #[zbus(signal, name = "layoutListChanged")]
    fn layout_list_changed(&self) -> zbus::Result<()>;
}

fn watch_plasma() -> zbus::Result<()> {
    let conn = zbus::blocking::Connection::session()?;
    let proxy = KeyboardLayoutsProxyBlocking::new(&conn)?;
    if let Ok(index) = proxy.get_layout() {
        PLASMA_GROUP.store(index, Ordering::Release);
        bump();
    }
    let list_proxy = proxy.clone();
    std::thread::spawn(move || {
        if let Ok(signals) = list_proxy.receive_layout_list_changed() {
            for _ in signals {
                bump();
            }
        }
    });
    for signal in proxy.receive_layout_changed()? {
        if let Ok(args) = signal.args() {
            PLASMA_GROUP.store(args.index, Ordering::Release);
            bump();
        }
    }
    Ok(())
}
