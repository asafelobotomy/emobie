//! Keyboard-access setup via pkexec (host path under Flatpak).
//!
//! Permanent access = group `emobie-input` + `/etc/udev/rules.d/99-emobie-input.rules`.
//! Ephemeral `can_listen` (ACL / orphaned GID) must not skip Grant.

mod permanent;
mod stage;

use permanent::{in_flatpak, permanent_access_gap_detail};
use stage::{ensure_polkit_annotated_setup, resolve_setup_script, run_pkexec};

use super::unix;
use super::InputHelperStatus;

pub use permanent::{host_setup_hint, permanent_access_configured};

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
        status.detail = "Keyboard access ready, but text injection needs writable /dev/uinput \
(run Grant again, install the acl package for setfacl, or log out/in so group emobie-input applies)."
            .into();
    } else if status.daemon && !status.can_inject {
        status.detail = "Helper running but text injection is unavailable (need /dev/uinput on Wayland). \
Run Grant or log out/in after setup-input-access.sh."
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
