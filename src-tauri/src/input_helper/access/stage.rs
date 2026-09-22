//! Resolve, stage, and run the Polkit-annotated setup script.
//!
//! Trust model: the only things ever executed or installed as root are
//! (a) the package-managed `/usr/share/emobie/setup-input-access.sh`, or
//! (b) the bytes embedded in this binary (see `assets`), staged into
//! `/usr/local/share/emobie/`. No user-writable file is ever copied to root.

use super::assets::{STAGED_FILES, UDEV_RULES as EMBEDDED_UDEV_RULES, UDEV_RULES_NAME};
use super::permanent::{host_setup_hint, in_flatpak, LOCAL_SETUP, SYSTEM_SETUP};
use std::io::Write;
use std::process::{Command, Stdio};

const LOCAL_DIR: &str = "/usr/local/share/emobie";
/// Directory `SYSTEM_SETUP` lives in — its sibling udev rule is how we decide
/// whether the distro package is current enough to trust for Grant.
const SYSTEM_DIR: &str = "/usr/share/emobie";

fn with_session_env(cmd: &mut Command) {
    for key in [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XAUTHORITY",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_RUNTIME_DIR",
    ] {
        if let Ok(value) = std::env::var(key) {
            cmd.env(key, value);
        }
    }
}

/// Root-owned file contents, or `None` when missing/unreadable. Only used to
/// decide whether a staged copy is already current — the data never gets
/// executed by this process.
fn read_installed(path: &str, flatpak: bool) -> Option<Vec<u8>> {
    let output = if flatpak {
        Command::new("flatpak-spawn")
            .args(["--host", "cat", path])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?
    } else {
        return std::fs::read(path).ok();
    };
    output.status.success().then_some(output.stdout)
}

/// `pkexec install -D -m MODE /dev/stdin DEST`, feeding `bytes` on stdin so the
/// data root installs is exactly what we hold in memory (no path to swap).
fn pkexec_install_bytes(flatpak: bool, mode: &str, bytes: &[u8], dest: &str) -> Result<(), String> {
    let install_args = ["install", "-D", "-m", mode, "/dev/stdin", dest];
    let mut cmd = if flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pkexec").args(install_args);
        c
    } else {
        let mut c = Command::new("pkexec");
        c.args(install_args);
        c
    };
    with_session_env(&mut cmd);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not stage {dest} ({e}). {}", host_setup_hint()))?;
    if let Some(mut stdin) = child.stdin.take() {
        // Errors surface through the exit status below (e.g. auth cancelled).
        let _ = stdin.write_all(bytes);
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Could not stage {dest} ({e}). {}", host_setup_hint()))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if detail.is_empty() {
        format!("Could not install {dest}. {}", host_setup_hint())
    } else {
        format!("Keyboard access setup staging ({dest}): {detail}")
    })
}

/// Stage every embedded asset that is missing or differs from the installed
/// copy into `/usr/local/share/emobie/` (root-owned).
fn stage_embedded_assets(flatpak: bool) -> Result<(), String> {
    for (bytes, mode, rel) in STAGED_FILES {
        let dest = format!("{LOCAL_DIR}/{rel}");
        if read_installed(&dest, flatpak).as_deref() == Some(bytes) {
            continue;
        }
        pkexec_install_bytes(flatpak, mode, bytes, &dest)?;
    }
    Ok(())
}

pub(super) fn ensure_polkit_annotated_setup(script: &str, flatpak: bool) -> Result<String, String> {
    if script == SYSTEM_SETUP {
        // Package-managed and root-owned: trusted as-is.
        return Ok(script.to_string());
    }
    stage_embedded_assets(flatpak)?;
    Ok(LOCAL_SETUP.to_string())
}

pub(super) fn run_pkexec(script: &str, flatpak: bool) -> Result<(), String> {
    let mut cmd = if flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pkexec").arg(script);
        c
    } else {
        let mut c = Command::new("pkexec");
        c.arg(script);
        c
    };
    with_session_env(&mut cmd);
    // pkexec scrubs the environment; the script resolves the invoking user
    // from PKEXEC_UID.
    let output = cmd.stdin(Stdio::null()).output().map_err(|e| {
        if flatpak {
            format!("flatpak-spawn --host failed ({e}). {}", host_setup_hint())
        } else {
            format!("Could not launch pkexec ({e}). {}", host_setup_hint())
        }
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or("cancelled or failed");
    Err(format!("Keyboard access setup: {detail}"))
}

/// True when the distro package's own udev rule (shipped beside `SYSTEM_SETUP`)
/// matches what this build embeds. A distro package can be older than a
/// Flatpak/AppImage installed alongside it; trusting a stale `SYSTEM_SETUP`
/// there would keep reinstalling its outdated rule while `permanent_access_configured`
/// (which compares against the *embedded* rule) forever reports the gap as
/// unrepaired. When the package is stale, fall through to staging our own
/// embedded copy under `/usr/local` instead.
fn system_package_current(flatpak: bool) -> bool {
    system_rules_match(&format!("{SYSTEM_DIR}/{UDEV_RULES_NAME}"), flatpak)
}

/// Testable core of `system_package_current`: does the udev rule at `path`
/// (read the same way `read_installed` reads any root-owned file) match what
/// this build embeds?
fn system_rules_match(path: &str, flatpak: bool) -> bool {
    read_installed(path, flatpak).as_deref() == Some(EMBEDDED_UDEV_RULES)
}

/// Which script to run as root: the package's, if installed *and current*,
/// otherwise our staged embedded copy (staged by `ensure_polkit_annotated_setup`).
pub(super) fn resolve_setup_script() -> Result<(String, bool), String> {
    let flatpak = in_flatpak();
    let system_present = if flatpak {
        super::permanent::host_file_exists(SYSTEM_SETUP)
    } else {
        std::path::Path::new(SYSTEM_SETUP).is_file()
    };
    let script = if system_present && system_package_current(flatpak) {
        SYSTEM_SETUP
    } else {
        LOCAL_SETUP
    };
    Ok((script.to_string(), flatpak))
}

#[cfg(test)]
mod tests {
    use super::super::assets::{POLKIT_POLICY, SELINUX_TE, SETUP_SCRIPT, STAGED_FILES, UDEV_RULES};

    #[test]
    fn embedded_assets_are_present_and_sane() {
        assert!(SETUP_SCRIPT.starts_with(b"#!/usr/bin/env bash"));
        assert!(UDEV_RULES.windows(6).any(|w| w == b"uinput"));
        assert!(POLKIT_POLICY.starts_with(b"<?xml"));
        assert!(!SELINUX_TE.is_empty());
        assert_eq!(STAGED_FILES.len(), 4);
    }

    #[test]
    fn shipped_udev_rule_grants_no_keyboard_read() {
        let text = String::from_utf8_lossy(UDEV_RULES);
        let active: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
            .collect();
        assert!(active.iter().all(|l| !l.contains("event*")), "{active:?}");
    }

    /// A distro package's own udev rule can be older than a Flatpak/AppImage
    /// installed alongside it. `resolve_setup_script` must not trust
    /// `SYSTEM_SETUP` in that case, or Grant could never repair the gap
    /// `permanent_access_configured` reports (it compares against the
    /// embedded rule, not whatever the stale package shipped).
    #[test]
    fn stale_system_package_is_not_trusted() {
        use super::system_rules_match;
        use std::fs;

        let dir = std::env::temp_dir().join(format!(
            "emobie-stage-test-{}-{}",
            std::process::id(),
            line!()
        ));
        fs::create_dir_all(&dir).unwrap();

        let current = dir.join("current.rules");
        fs::write(&current, UDEV_RULES).unwrap();
        assert!(
            system_rules_match(current.to_str().unwrap(), false),
            "byte-identical rule must be trusted"
        );

        let stale = dir.join("stale.rules");
        fs::write(&stale, b"# an older, different rule\n").unwrap();
        assert!(
            !system_rules_match(stale.to_str().unwrap(), false),
            "a package shipping a different rule must not be trusted"
        );

        let missing = dir.join("missing.rules");
        assert!(
            !system_rules_match(missing.to_str().unwrap(), false),
            "a missing sibling rule must not be trusted"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
