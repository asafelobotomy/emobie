//! Activation: `copy` puts the emoji on the clipboard with `wl-copy`; `type` copies it and then
//! asks emobie-inputd to paste it into the focused window, or stays a copy when the helper isn't
//! running. Nothing from the bus reaches a shell: the emoji is one argv entry, and only emoji
//! from the catalog are accepted.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_WL_COPY: &str = "/usr/bin/wl-copy";
/// The longest emoji in emojibase is well under this (ZWJ sequences with skin tones).
pub const MAX_ID_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Copy,
    Type,
}

pub const ACTIONS_JSON: &str = r#"["copy","type"]"#;

impl Action {
    /// `""` is uQ's "the default action" (Enter).
    pub fn parse(action: &str) -> Result<Self, String> {
        match action {
            "" | "copy" => Ok(Action::Copy),
            "type" => Ok(Action::Type),
            other => {
                let shown: String = other.chars().take(32).collect();
                Err(format!("unknown action {shown:?}"))
            }
        }
    }
}

/// Runs `wl-copy -- <text>` and waits for it to hand the text to its background copy.
#[derive(Debug, Clone)]
pub struct Clipboard {
    pub program: PathBuf,
    pub timeout: Duration,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            program: PathBuf::from(DEFAULT_WL_COPY),
            timeout: Duration::from_secs(2),
        }
    }
}

impl Clipboard {
    pub fn copy(&self, text: &str) -> Result<(), String> {
        let name = self.program.display();
        let mut child = Command::new(&self.program)
            .arg("--")
            .arg(text)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("{name}: {e}"))?;
        // wl-copy forks a process that serves the selection and exits; wait for that, briefly.
        let deadline = Instant::now() + self.timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => return Err(format!("{name} failed ({status})")),
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{name} timed out"));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(e) => return Err(format!("{name}: {e}")),
            }
        }
    }
}

/// Pastes the clipboard into the focused window.
pub trait Paster: Send + Sync {
    /// Whether a paste can be attempted at all (the helper is running).
    fn available(&self) -> bool;
    fn paste(&self) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn parses_actions() {
        assert_eq!(Action::parse(""), Ok(Action::Copy));
        assert_eq!(Action::parse("copy"), Ok(Action::Copy));
        assert_eq!(Action::parse("type"), Ok(Action::Type));
        assert!(Action::parse("Copy").is_err());
        assert!(Action::parse("rm -rf").is_err());
        assert!(Action::parse(&"x".repeat(5000)).unwrap_err().len() < 64);
    }

    fn stub(dir: &std::path::Path, body: &str) -> PathBuf {
        let path = dir.join("wl-copy");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "emobie-provider-activate-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn copies_with_argv_and_no_shell() {
        let dir = tempdir("argv");
        let out = dir.join("args");
        let program = stub(
            &dir,
            &format!(
                "for a in \"$@\"; do printf '%s\\n' \"$a\"; done > '{}'",
                out.display()
            ),
        );
        let clip = Clipboard {
            program,
            timeout: Duration::from_secs(5),
        };
        clip.copy("👋🏽").unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "--\n👋🏽\n");
        // Shell syntax stays one literal argument.
        clip.copy("$(touch pwned); `id`").unwrap();
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "--\n$(touch pwned); `id`\n"
        );
        assert!(!dir.join("pwned").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reports_failure_missing_program_and_timeout() {
        let dir = tempdir("fail");
        let failing = Clipboard {
            program: stub(&dir, "exit 3"),
            timeout: Duration::from_secs(5),
        };
        assert!(failing.copy("x").unwrap_err().contains("failed"));
        let missing = Clipboard {
            program: dir.join("absent"),
            timeout: Duration::from_secs(1),
        };
        assert!(missing.copy("x").is_err());
        let slow_dir = dir.join("slow");
        std::fs::create_dir_all(&slow_dir).unwrap();
        let slow = Clipboard {
            program: stub(&slow_dir, "exec sleep 5"),
            timeout: Duration::from_millis(200),
        };
        let started = Instant::now();
        assert!(slow.copy("x").unwrap_err().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(3));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
