//! Keyboard-access setup via pkexec (host path under Flatpak).
//!
//! Permanent access = group `emobie-input` + `/etc/udev/rules.d/99-emobie-input.rules`.
//! Ephemeral `can_listen` (ACL / orphaned GID) must not skip Grant.

use super::unix;
use super::InputHelperStatus;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

const SYSTEM_SETUP: &str = "/usr/share/emobie/setup-input-access.sh";
const LOCAL_SETUP: &str = "/usr/local/share/emobie/setup-input-access.sh";
const LOCAL_DIR: &str = "/usr/local/share/emobie";
const UDEV_RULES: &str = "/etc/udev/rules.d/99-emobie-input.rules";
const GROUP_NAME: &str = "emobie-input";

fn in_flatpak() -> bool {
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

fn user_setup_script() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h).join(".local/share/emobie/setup-input-access.sh")
    })
}

fn user_data_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/emobie"))
}

fn sandbox_setup_scripts() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let local_helper = home
        .as_ref()
        .map(|h| h.join(".local/bin/emobie-inputd"))
        .filter(|p| p.is_file());
    // Prefer user/local copies when the host helper is under ~/.local/bin
    // (AppImage/Flatpak bootstrap) so Grant does not run a stale system script.
    if local_helper.is_some() {
        // Prefer Polkit-annotated /usr/local copy when present (Grant stages it).
        paths.push(PathBuf::from(LOCAL_SETUP));
        if let Some(user) = user_setup_script() {
            paths.push(user);
        }
        paths.push(PathBuf::from(SYSTEM_SETUP));
    } else {
        paths.push(PathBuf::from(SYSTEM_SETUP));
        paths.push(PathBuf::from(LOCAL_SETUP));
        if let Some(user) = user_setup_script() {
            paths.push(user);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("setup-input-access.sh"));
            paths.push(dir.join("../../../packaging/setup-input-access.sh"));
        }
    }
    paths
}

fn host_setup_candidates() -> Vec<String> {
    let mut paths = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let local_bin = format!("{home}/.local/bin/emobie-inputd");
        let user_setup = format!("{home}/.local/share/emobie/setup-input-access.sh");
        if host_file_exists(&local_bin) {
            paths.push(LOCAL_SETUP.to_string());
            paths.push(user_setup);
            paths.push(SYSTEM_SETUP.to_string());
            return paths;
        }
        paths.push(SYSTEM_SETUP.to_string());
        paths.push(LOCAL_SETUP.to_string());
        paths.push(user_setup);
    } else {
        paths.push(SYSTEM_SETUP.to_string());
        paths.push(LOCAL_SETUP.to_string());
    }
    paths
}

fn host_file_exists(path: &str) -> bool {
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

fn permanent_access_gap_detail() -> String {
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

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Pick the newest existing script among candidates (avoids stale packaged Grant).
fn newest_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for path in paths {
        if !path.is_file() {
            continue;
        }
        let Some(modified) = mtime(path) else {
            continue;
        };
        match &best {
            Some((t, _)) if *t >= modified => {}
            _ => best = Some((modified, path.clone())),
        }
    }
    best.map(|(_, p)| p)
}

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

fn is_polkit_annotated_path(script: &str) -> bool {
    script == SYSTEM_SETUP || script == LOCAL_SETUP
}

fn pkexec_install(flatpak: bool, mode: &str, source: &str, dest: &str) -> Result<(), String> {
    let install_args = ["install", "-D", "-m", mode, source, dest];
    let output = if flatpak {
        let mut cmd = Command::new("flatpak-spawn");
        cmd.arg("--host").arg("pkexec").args(install_args);
        with_session_env(&mut cmd);
        cmd.stdin(Stdio::null()).output()
    } else {
        let mut cmd = Command::new("pkexec");
        cmd.args(install_args);
        with_session_env(&mut cmd);
        cmd.stdin(Stdio::null()).output()
    }
    .map_err(|e| format!("Could not stage {dest} ({e}). {}", host_setup_hint()))?;

    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr)
        .trim()
        .to_string();
    Err(if detail.is_empty() {
        format!("Could not install {dest}. {}", host_setup_hint())
    } else {
        format!("Keyboard access setup staging ({dest}): {detail}")
    })
}

/// Copy setup script + udev/policy siblings to the Polkit-annotated local path.
fn stage_setup_to_local(source: &str, flatpak: bool) -> Result<(), String> {
    if is_polkit_annotated_path(source) {
        // Still ensure udev rules sit beside LOCAL_SETUP for AppImage re-Grants.
        if source == LOCAL_SETUP {
            stage_local_siblings(source, flatpak)?;
        }
        return Ok(());
    }
    pkexec_install(flatpak, "755", source, LOCAL_SETUP)?;
    stage_local_siblings(source, flatpak)?;
    Ok(())
}

fn stage_local_siblings(setup_source: &str, flatpak: bool) -> Result<(), String> {
    let setup_path = Path::new(setup_source);
    let sibling_dir = setup_path
        .parent()
        .map(Path::to_path_buf)
        .or_else(user_data_dir);

    let rules_candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Some(dir) = &sibling_dir {
            v.push(dir.join("99-emobie-input.rules"));
        }
        if let Some(dir) = user_data_dir() {
            v.push(dir.join("99-emobie-input.rules"));
        }
        v.push(PathBuf::from("/usr/share/emobie/99-emobie-input.rules"));
        v
    };
    let rules_src = rules_candidates.into_iter().find(|p| {
        if flatpak {
            host_file_exists(&p.to_string_lossy())
        } else {
            p.is_file()
        }
    });
    if let Some(rules) = rules_src {
        let dest = format!("{LOCAL_DIR}/99-emobie-input.rules");
        let missing = if flatpak {
            !host_file_exists(&dest)
        } else {
            !Path::new(&dest).is_file()
        };
        if missing {
            pkexec_install(flatpak, "644", &rules.to_string_lossy(), &dest)?;
        }
    }

    let policy_candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Some(dir) = &sibling_dir {
            v.push(dir.join("io.github.asafelobotomy.emobie.inputd.policy"));
        }
        if let Some(dir) = user_data_dir() {
            v.push(dir.join("io.github.asafelobotomy.emobie.inputd.policy"));
        }
        v
    };
    let policy_src = policy_candidates.into_iter().find(|p| {
        if flatpak {
            host_file_exists(&p.to_string_lossy())
        } else {
            p.is_file()
        }
    });
    if let Some(policy) = policy_src {
        let dest = format!("{LOCAL_DIR}/io.github.asafelobotomy.emobie.inputd.policy");
        let missing = if flatpak {
            !host_file_exists(&dest)
        } else {
            !Path::new(&dest).is_file()
        };
        if missing {
            let _ = pkexec_install(flatpak, "644", &policy.to_string_lossy(), &dest);
        }
    }

    Ok(())
}

fn ensure_polkit_annotated_setup(script: &str, flatpak: bool) -> Result<String, String> {
    if is_polkit_annotated_path(script) {
        if script == LOCAL_SETUP {
            stage_local_siblings(script, flatpak)?;
        }
        return Ok(script.to_string());
    }
    stage_setup_to_local(script, flatpak)?;
    Ok(LOCAL_SETUP.to_string())
}

fn pkexec_args(script: &str) -> Vec<String> {
    vec![script.to_string()]
}

fn run_pkexec(script: &str, flatpak: bool) -> Result<(), String> {
    let args = pkexec_args(script);
    let mut cmd = if flatpak {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pkexec").args(&args);
        c
    } else {
        let mut c = Command::new("pkexec");
        c.args(&args);
        c
    };
    with_session_env(&mut cmd);
    // Ensure setup script can resolve the invoking user on all distros.
    if let Ok(user) = std::env::var("USER") {
        cmd.env("SUDO_USER", user);
    }

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

fn resolve_setup_script() -> Result<(String, bool), String> {
    let flatpak = in_flatpak();
    if flatpak {
        for path in host_setup_candidates() {
            if host_file_exists(&path) {
                return Ok((path, true));
            }
        }
        // Never pass sandbox (/app) paths to --host pkexec — the host cannot read them.
        return Err(host_setup_hint());
    }

    let candidates = sandbox_setup_scripts();
    let script = newest_existing(&candidates).ok_or_else(|| {
        "setup-input-access.sh not found — install the emobie package or run \
packaging/setup-input-access.sh"
            .to_string()
    })?;
    Ok((script.to_string_lossy().into_owned(), false))
}

pub fn with_flatpak_flag(mut status: InputHelperStatus) -> InputHelperStatus {
    status.flatpak = in_flatpak();
    status.access_configured = permanent_access_configured();
    if !status.daemon
        && status.flatpak
        && !status.detail.contains("Flatpak needs a host helper")
    {
        status.detail = format!(
            "{} Enable Expand to install the host helper automatically.",
            status.detail
        );
    }
    // Surface permanent gaps even when ephemeral listen already works.
    if status.daemon && status.can_listen && !status.access_configured {
        let gap = permanent_access_gap_detail();
        if !status.detail.contains("permanent keyboard access") {
            status.detail = format!("{} — {gap}", status.detail);
        }
    }
    status
}

/// Polkit setup + restart helper; returns fresh status (can_listen after ACLs).
pub fn run_access_setup() -> Result<InputHelperStatus, String> {
    let (script, flatpak) = resolve_setup_script()?;
    let script = ensure_polkit_annotated_setup(&script, flatpak)?;
    run_pkexec(&script, flatpak)?;
    let mut status = with_flatpak_flag(unix::restart_helper());
    if !status.access_configured {
        status.detail = format!(
            "Grant finished but {} — {}",
            permanent_access_gap_detail(),
            host_setup_hint()
        );
    } else if status.can_listen && status.can_inject {
        status.detail =
            "Keyboard access ready — Expand as you type can watch keys and inject text.".into();
    } else if status.can_listen {
        status.detail = "Keyboard access ready, but text injection needs a desktop session. \
Restart emobie-inputd from your graphical session (or log out/in)."
            .into();
    } else if status.daemon && !status.can_inject {
        status.detail = "Helper running but text injection is unavailable (no compositor env). \
Log out/in or restart emobie-inputd from a graphical session."
            .into();
    } else if status.daemon {
        status.detail = "Helper restarted but keyboard devices are still closed. \
If session ACLs failed, log out/in once so the emobie-input group applies."
            .into();
    } else {
        status.detail = format!(
            "Access script finished but helper is not running. {}",
            status.detail
        );
    }
    Ok(status)
}
