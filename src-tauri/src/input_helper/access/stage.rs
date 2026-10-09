//! Resolve, stage, and run the Polkit-annotated setup script.
//!
//! Trust model: the only things ever executed or installed as root are
//! (a) the package-managed `/usr/share/emobie/setup-input-access.sh`, or
//! (b) the bytes embedded in this binary (see `assets`), staged into
//! `/usr/local/share/emobie/`. No user-writable file is ever copied to root.

use super::assets::{
    KEYBOARD_READ_RULES, KEYBOARD_READ_RULES_NAME, STAGED_FILES, UDEV_RULES as EMBEDDED_UDEV_RULES,
    UDEV_RULES_NAME,
};
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

/// Root half of a Grant that has to (re)stage files: reads `REL MODE BASE64`
/// lines on stdin, installs each into `LOCAL_DIR`, then runs the staged setup
/// script with this command's arguments. Staging and setup share one `pkexec`
/// call — one password prompt — and the bytes come straight from this
/// process's memory (no path a user process could swap).
const STAGE_AND_RUN: &str = r#"set -eu
dir=/usr/local/share/emobie
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
while read -r rel mode data; do
  case "$rel" in ''|/*|*..*) echo "emobie: refusing staged path '$rel'" >&2; exit 1 ;; esac
  case "$mode" in 644|755) ;; *) echo "emobie: refusing mode '$mode'" >&2; exit 1 ;; esac
  printf '%s' "$data" | base64 -d > "$tmp/file"
  install -D -m "$mode" "$tmp/file" "$dir/$rel"
done
"$dir/setup-input-access.sh" "$@"
"#;

/// Standard base64 (RFC 4648, padded) for the `STAGE_AND_RUN` payload.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &b)| acc | (u32::from(b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Embedded assets whose installed copy under `LOCAL_DIR` is missing or
/// differs, as `STAGE_AND_RUN` input.
fn stale_assets_payload(flatpak: bool) -> String {
    STAGED_FILES
        .iter()
        .filter(|(bytes, _, rel)| {
            read_installed(&format!("{LOCAL_DIR}/{rel}"), flatpak).as_deref() != Some(*bytes)
        })
        .map(|(bytes, mode, rel)| format!("{rel} {mode} {}\n", base64(bytes)))
        .collect()
}

/// Run the setup script as root with `args`: the package's copy, or our
/// embedded copy staged under `LOCAL_DIR` (staged in the same prompt when it
/// is missing or out of date).
pub(super) fn run_setup(args: &[&str]) -> Result<(), String> {
    let (script, flatpak) = resolve_setup_script()?;
    if script == SYSTEM_SETUP {
        // Package-managed and root-owned: trusted as-is.
        return run_pkexec(&[SYSTEM_SETUP], args, flatpak, None);
    }
    let payload = stale_assets_payload(flatpak);
    if payload.is_empty() {
        // Already current: run it directly so Polkit shows emobie's own action.
        return run_pkexec(&[LOCAL_SETUP], args, flatpak, None);
    }
    run_pkexec(
        &["/bin/sh", "-c", STAGE_AND_RUN, "emobie-grant"],
        args,
        flatpak,
        Some(payload.as_bytes()),
    )
}

fn run_pkexec(
    command: &[&str],
    args: &[&str],
    flatpak: bool,
    stdin: Option<&[u8]>,
) -> Result<(), String> {
    let mut cmd = if flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pkexec");
        c
    } else {
        Command::new("pkexec")
    };
    cmd.args(command).args(args);
    with_session_env(&mut cmd);
    // pkexec scrubs the environment; the script resolves the invoking user
    // from PKEXEC_UID.
    cmd.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let launch_error = |e: std::io::Error| {
        if flatpak {
            format!("flatpak-spawn --host failed ({e}). {}", host_setup_hint())
        } else {
            format!("Could not launch pkexec ({e}). {}", host_setup_hint())
        }
    };
    let mut child = cmd.spawn().map_err(launch_error)?;
    if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
        // Errors surface through the exit status below (e.g. auth cancelled).
        let _ = pipe.write_all(bytes);
    }
    let output = child.wait_with_output().map_err(launch_error)?;

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
        && read_installed(&format!("{SYSTEM_DIR}/{KEYBOARD_READ_RULES_NAME}"), flatpak).as_deref()
            == Some(KEYBOARD_READ_RULES)
}

/// Testable core of `system_package_current`: does the udev rule at `path`
/// (read the same way `read_installed` reads any root-owned file) match what
/// this build embeds?
fn system_rules_match(path: &str, flatpak: bool) -> bool {
    read_installed(path, flatpak).as_deref() == Some(EMBEDDED_UDEV_RULES)
}

/// Which script to run as root: the package's, if installed *and current*,
/// otherwise our staged embedded copy (staged by `run_setup`).
fn resolve_setup_script() -> Result<(String, bool), String> {
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
    #[test]
    fn base64_matches_rfc4648_vectors() {
        use super::base64;
        for (raw, enc) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(raw.as_bytes()), enc);
        }
    }

    /// Run the real root-side staging script (pointed at a temp dir, with the
    /// final setup call stubbed out) over the payload for every embedded asset,
    /// and check each file lands byte-for-byte with its mode.
    #[test]
    fn stage_and_run_installs_exact_bytes() {
        use super::{base64, STAGE_AND_RUN};
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};

        let dir = std::env::temp_dir().join(format!("emobie-stage-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let script = STAGE_AND_RUN
            .replace("dir=/usr/local/share/emobie", &format!("dir='{}'", dir.display()))
            .replace("\"$dir/setup-input-access.sh\" \"$@\"", "echo ran \"$@\"");
        let payload: String = STAGED_FILES
            .iter()
            .map(|(bytes, mode, rel)| format!("{rel} {mode} {}\n", base64(bytes)))
            .collect();
        let mut child = Command::new("sh")
            .args(["-c", &script, "emobie-grant", "--keyboard-read", "on"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(payload.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ran --keyboard-read on");
        for (bytes, mode, rel) in STAGED_FILES {
            let path = dir.join(rel);
            assert_eq!(std::fs::read(&path).unwrap(), bytes, "{rel}");
            let got = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(format!("{got:o}"), mode, "{rel}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stage_and_run_refuses_path_escapes() {
        use super::STAGE_AND_RUN;
        use std::io::Write;
        use std::process::{Command, Stdio};

        let dir = std::env::temp_dir().join(format!("emobie-stage-esc-{}", std::process::id()));
        let script = STAGE_AND_RUN
            .replace("dir=/usr/local/share/emobie", &format!("dir='{}'", dir.display()));
        for line in ["../evil 644 eA==\n", "/etc/evil 644 eA==\n", "ok 4755 eA==\n"] {
            let mut child = Command::new("sh")
                .args(["-c", &script, "emobie-grant"])
                .stdin(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(line.as_bytes()).unwrap();
            assert!(!child.wait().unwrap().success(), "{line:?} must be refused");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    use super::super::assets::{POLKIT_POLICY, SELINUX_TE, SETUP_SCRIPT, STAGED_FILES, UDEV_RULES};

    #[test]
    fn embedded_assets_are_present_and_sane() {
        assert!(SETUP_SCRIPT.starts_with(b"#!/usr/bin/env bash"));
        assert!(UDEV_RULES.windows(6).any(|w| w == b"uinput"));
        assert!(POLKIT_POLICY.starts_with(b"<?xml"));
        assert!(!SELINUX_TE.is_empty());
        assert_eq!(STAGED_FILES.len(), 5);
    }

    /// Keyboard read must come from the opt-in rule only, and only as an ACL:
    /// `GROUP=` on event nodes would take them away from the `input` group.
    #[test]
    fn keyboard_read_rule_uses_acls_only() {
        let text = String::from_utf8_lossy(super::super::assets::KEYBOARD_READ_RULES);
        let active: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
            .collect();
        assert!(active.iter().all(|l| !l.contains("GROUP=") && !l.contains("MODE=")), "{active:?}");
        assert!(active.iter().any(|l| l.contains("setfacl -m g:emobie-input:r")));
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
