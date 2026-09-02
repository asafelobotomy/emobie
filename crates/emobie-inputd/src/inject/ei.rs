//! Optional Emulated Input (libei) / eitype insert for complex Unicode text.
//!
//! Feature `libei` enables a native reis/ashpd hook. Prefer a system `eitype`
//! binary on PATH when available — it already owns the RemoteDesktop portal UX.
//! Do not enable Enigo's bundled libei path (0.3 panics without portal consent).

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Try to type `body` without clipboard.
///
/// Order: optional `eitype` on PATH (system pack), then feature-gated reis/libei.
pub fn try_type_without_clipboard(body: &str) -> Result<(), String> {
    if let Ok(()) = try_eitype(body) {
        return Ok(());
    }
    #[cfg(feature = "libei")]
    {
        return ei_native::try_type(body);
    }
    #[cfg(not(feature = "libei"))]
    {
        Err("no EI typer available (install eitype or build with --features libei)".into())
    }
}

fn try_eitype(body: &str) -> Result<(), String> {
    let mut child = Command::new("eitype")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("eitype spawn: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(body.as_bytes())
            .map_err(|e| format!("eitype write: {e}"))?;
    }
    let status = wait_with_timeout(&mut child, Duration::from_millis(2_000))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("eitype exit {status}"))
    }
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, String> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("eitype timed out".into());
            }
            Err(e) => return Err(format!("eitype wait: {e}")),
        }
    }
}

#[cfg(feature = "libei")]
mod ei_native {
    //! Native reis + ashpd RemoteDesktop path (feature-gated).
    //!
    //! Full portal session management belongs in a follow-up: request
    //! RemoteDesktop, connect EIS, open a keyboard device, then type. Until
    //! that lands, clipboard remains the Unicode fallback when eitype is absent.

    #[allow(dead_code)]
    fn _deps_linked() {
        // Keep optional crates referenced so `--features libei` stays compile-checked.
        let _ = std::any::type_name::<reis::ei::Context>();
        let _ = std::any::type_name::<ashpd::desktop::remote_desktop::RemoteDesktop>();
    }

    pub fn try_type(body: &str) -> Result<(), String> {
        let _ = body;
        let _ = _deps_linked;
        Err(
            "native libei inject requires an active RemoteDesktop portal session; \
             use eitype or clipboard for now"
                .into(),
        )
    }
}
