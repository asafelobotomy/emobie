//! Permanent keyboard-access detection (group + udev rules).

use std::path::Path;
use std::process::{Command, Stdio};

pub(super) const SYSTEM_SETUP: &str = "/usr/share/emobie/setup-input-access.sh";
pub(super) const LOCAL_SETUP: &str = "/usr/local/share/emobie/setup-input-access.sh";
const UDEV_RULES: &str = "/etc/udev/rules.d/99-emobie-input.rules";
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

/// True when group + udev rules are permanently configured (survives reboot).
pub fn permanent_access_configured() -> bool {
    if in_flatpak() {
        return host_cmd_succeeds(&["getent", "group", GROUP_NAME])
            && host_file_exists(UDEV_RULES);
    }
    local_cmd_succeeds("getent", &["group", GROUP_NAME]) && Path::new(UDEV_RULES).is_file()
}

pub(super) fn permanent_access_gap_detail() -> String {
    let mut missing = Vec::new();
    let group_ok = if in_flatpak() {
        host_cmd_succeeds(&["getent", "group", GROUP_NAME])
    } else {
        local_cmd_succeeds("getent", &["group", GROUP_NAME])
    };
    let rules_ok = if in_flatpak() {
        host_file_exists(UDEV_RULES)
    } else {
        Path::new(UDEV_RULES).is_file()
    };
    if !group_ok {
        missing.push(format!("group `{GROUP_NAME}`"));
    }
    if !rules_ok {
        missing.push(format!("udev rules `{UDEV_RULES}`"));
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
