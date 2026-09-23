//! Permanent keyboard-access detection (group + udev rules).

use super::assets::{KEYBOARD_READ_RULES, UDEV_RULES as EMBEDDED_UDEV_RULES};
use std::process::{Command, Stdio};

pub(super) const SYSTEM_SETUP: &str = "/usr/share/emobie/setup-input-access.sh";
pub(super) const LOCAL_SETUP: &str = "/usr/local/share/emobie/setup-input-access.sh";
pub(super) const UDEV_RULES_PATH: &str = "/etc/udev/rules.d/99-emobie-input.rules";
const KEYBOARD_READ_RULES_PATH: &str = "/etc/udev/rules.d/98-emobie-keyboard-read.rules";
const GROUP_NAME: &str = "emobie-input";

pub(super) fn in_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

pub fn host_setup_hint() -> String {
    if in_flatpak() {
        format!(
            "Flatpak installs the host input helper when you enable Expand. \
If Grant fails, run on the host: pkexec {LOCAL_SETUP}"
        )
    } else {
        format!("Run: pkexec {LOCAL_SETUP} (or {SYSTEM_SETUP}), then retry.")
    }
}

pub(super) fn host_file_exists(path: &str) -> bool {
    Command::new("flatpak-spawn")
        .args(["--host", "test", "-f", path])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn host_cmd_succeeds(args: &[&str]) -> bool {
    Command::new("flatpak-spawn")
        .arg("--host")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn local_cmd_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_root_file(path: &str) -> Option<Vec<u8>> {
    if in_flatpak() {
        Command::new("flatpak-spawn")
            .args(["--host", "cat", path])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| o.stdout)
    } else {
        std::fs::read(path).ok()
    }
}

/// True when the opt-in keyboard-read rule ("Expand as you type") is
/// installed and matches this build.
pub fn keyboard_read_configured() -> bool {
    read_root_file(KEYBOARD_READ_RULES_PATH).as_deref() == Some(KEYBOARD_READ_RULES)
}

/// True when the installed udev rule is byte-identical to the one this build
/// ships. Existence alone is not enough: an old rule that still grants keyboard
/// *read* access would otherwise be reported as "configured" forever.
fn udev_rules_current() -> bool {
    read_root_file(UDEV_RULES_PATH).as_deref() == Some(EMBEDDED_UDEV_RULES)
}

fn udev_rules_present() -> bool {
    if in_flatpak() {
        host_file_exists(UDEV_RULES_PATH)
    } else {
        std::path::Path::new(UDEV_RULES_PATH).is_file()
    }
}

/// True when group + udev rules are permanently configured (survives reboot).
pub fn permanent_access_configured() -> bool {
    let group_ok = if in_flatpak() {
        host_cmd_succeeds(&["getent", "group", GROUP_NAME])
    } else {
        local_cmd_succeeds("getent", &["group", GROUP_NAME])
    };
    group_ok && udev_rules_current()
}

pub(super) fn permanent_access_gap_detail() -> String {
    let mut missing = Vec::new();
    let group_ok = if in_flatpak() {
        host_cmd_succeeds(&["getent", "group", GROUP_NAME])
    } else {
        local_cmd_succeeds("getent", &["group", GROUP_NAME])
    };
    let rules_missing = !udev_rules_present();
    let rules_ok = !rules_missing && udev_rules_current();
    if !group_ok {
        missing.push(format!("group `{GROUP_NAME}`"));
    }
    if !rules_ok {
        missing.push(if rules_missing {
            format!("udev rules `{UDEV_RULES_PATH}`")
        } else {
            format!("current udev rules (`{UDEV_RULES_PATH}` is outdated)")
        });
    }
    if missing.is_empty() {
        "permanent keyboard access looks configured".into()
    } else {
        format!(
            "permanent keyboard access incomplete (missing {}) — Grant will repair",
            missing.join(" and ")
        )
    }
}
