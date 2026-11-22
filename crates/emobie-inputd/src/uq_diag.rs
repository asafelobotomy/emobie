//! uq-diag 1.0.1
//!
//! uQ's log format and crash records for a Rust program. Vendor this file unchanged (see uQ's
//! `native/uq-diag/README.md`); it needs only `tracing`, `tracing-subscriber` (with `env-filter`)
//! and `serde_json`.
//!
//! - [`init`] sets up tracing the way every uQ program logs: under systemd each line starts with
//!   its syslog priority (`<4>bindle: [BINDLE-003] …`), so journald files it at the right level;
//!   in a terminal it reads `WARN bindle: [BINDLE-003] …`.
//! - Its panic hook saves a crash record (`crash.json`) in uQ's crash folder,
//!   `$XDG_STATE_HOME/uq/crashes/<program>-<time>-<pid>/`. uQ notices it, says so in a
//!   notification and lists it on Settings › Problems with a report people can copy.
//!
//! Start every warning and error with your app's code in brackets, `[BINDLE-003] …`, and list the
//! codes in `/usr/share/uq/diag/<app>.toml` (layout in the README) so `uqctl explain` and the
//! Problems page can say what each one means.

use std::fmt;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// The version of this file; `check-uq-diag.sh` compares it with the one a repo pins.
pub const VERSION: &str = "1.0.1";

/// How lines look: [`Style::Journal`] under systemd, [`Style::Terminal`] otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Journal,
    Terminal,
}

impl Style {
    /// `Journal` when stderr is systemd's journal stream (`JOURNAL_STREAM` set, not a terminal).
    pub fn detect() -> Style {
        if std::env::var_os("JOURNAL_STREAM").is_some() && !std::io::stderr().is_terminal() {
            Style::Journal
        } else {
            Style::Terminal
        }
    }
}

/// The syslog priority journald stores for a level.
pub fn priority(level: &Level) -> u8 {
    match *level {
        Level::ERROR => 3,
        Level::WARN => 4,
        Level::INFO => 6,
        _ => 7,
    }
}

/// `<p>program: message fields` for the journal, `LEVEL program: message fields` for a person.
/// The program's own modules (and uQ's library) read as the program; others by their crate.
pub struct UqFormat {
    pub style: Style,
    pub program: &'static str,
}

impl<S, N> FormatEvent<S, N> for UqFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut w: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        match self.style {
            Style::Journal => write!(w, "<{}>", priority(meta.level()))?,
            Style::Terminal => write!(w, "{:>5} ", meta.level().as_str())?,
        }
        let krate = meta.target().split("::").next().unwrap_or("");
        let own = krate.replace('_', "-") == self.program.replace('_', "-")
            || krate == "uq_core"
            || krate == self.program.split('-').next().unwrap_or("");
        write!(
            w,
            "{}: ",
            if own || krate.is_empty() {
                self.program
            } else {
                krate
            }
        )?;
        ctx.format_fields(w.by_ref(), event)?;
        writeln!(w)
    }
}

/// Set up tracing for `program`: `RUST_LOG`, or `default_filter` without it (e.g. "info"), and
/// [`install_panic_hook`].
pub fn init(program: &'static str, default_filter: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    init_with(program, filter);
}

/// [`init`] with a filter the program built itself.
pub fn init_with(program: &'static str, filter: EnvFilter) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .event_format(UqFormat {
            style: Style::detect(),
            program,
        })
        .try_init();
    install_panic_hook(program);
}

/// Whether `s` is a code: an upper-case area (letters and digits), a dash and three digits.
pub fn is_code(s: &str) -> bool {
    let Some((a, n)) = s.split_once('-') else {
        return false;
    };
    !a.is_empty()
        && a.len() <= 12
        && a.starts_with(|c: char| c.is_ascii_uppercase())
        && a.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && n.len() == 3
        && n.chars().all(|c| c.is_ascii_digit())
}

/// The first `[AREA-NNN]` in a message.
pub fn code_in(msg: &str) -> Option<&str> {
    let mut rest = msg;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        // No let chains: apps on edition 2021 vendor this file too.
        if let Some(code) = after
            .find(']')
            .map(|close| &after[..close])
            .filter(|c| is_code(c))
        {
            return Some(code);
        }
        rest = after;
    }
    None
}

/// `$XDG_STATE_HOME/uq/crashes` (only an absolute `XDG_STATE_HOME` counts), else
/// `~/.local/state/uq/crashes`.
pub fn crash_dir() -> PathBuf {
    crash_dir_from(&|k| std::env::var(k).ok())
}

/// [`crash_dir`] from an environment lookup.
pub fn crash_dir_from(get: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    let state = get("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(get("HOME").unwrap_or_else(|| "/tmp".into())).join(".local/state")
        });
    state.join("uq/crashes")
}

/// What a panic leaves behind, as `crash.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanicRecord {
    pub program: String,
    pub version: String,
    /// Seconds since the epoch.
    pub time: i64,
    pub pid: u32,
    pub thread: String,
    pub message: String,
    /// `file:line`.
    pub location: String,
    pub backtrace: String,
}

impl PanicRecord {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "program": self.program,
            "version": self.version,
            "time": self.time,
            "pid": self.pid,
            "thread": self.thread,
            "message": self.message,
            "location": self.location,
            "backtrace": self.backtrace,
        })
    }

    /// A `crash.json`; `None` when it isn't one.
    pub fn from_json(v: &serde_json::Value) -> Option<PanicRecord> {
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        Some(PanicRecord {
            program: s("program")?,
            version: s("version").unwrap_or_default(),
            time: v.get("time")?.as_i64()?,
            pid: v.get("pid").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            thread: s("thread").unwrap_or_default(),
            message: s("message").unwrap_or_default(),
            location: s("location").unwrap_or_default(),
            backtrace: s("backtrace").unwrap_or_default(),
        })
    }
}

/// Write `rec` as `<dir>/<program>-<time>-<pid>/crash.json` (through a temporary file, so uQ
/// never reads half of one); the folder's path.
pub fn save_panic(dir: &Path, rec: &PanicRecord) -> std::io::Result<PathBuf> {
    let folder = dir.join(format!("{}-{}-{}", rec.program, rec.time, rec.pid));
    std::fs::create_dir_all(&folder)?;
    let tmp = folder.join(".crash.json.tmp");
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(
        serde_json::to_string_pretty(&rec.to_json())
            .map_err(std::io::Error::other)?
            .as_bytes(),
    )?;
    f.sync_all()?;
    std::fs::rename(&tmp, folder.join("crash.json"))?;
    Ok(folder)
}

/// On a panic: save a [`PanicRecord`] and log it as `[CRASH-004]` (uQ's code for "a program
/// stopped on an internal error"), then run the default hook, which prints the message as
/// before. Installed once per process; [`init`] calls it.
pub fn install_panic_hook(program: &'static str) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "(no message)".to_string()
            };
            let rec = PanicRecord {
                program: program.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64),
                pid: std::process::id(),
                thread: std::thread::current().name().unwrap_or("?").to_string(),
                message,
                location: info
                    .location()
                    .map(|l| format!("{}:{}", l.file(), l.line()))
                    .unwrap_or_default(),
                backtrace: std::backtrace::Backtrace::force_capture().to_string(),
            };
            match save_panic(&crash_dir(), &rec) {
                Ok(p) => tracing::error!(
                    "[CRASH-004] {program} stopped on an internal error at {}: {}; report in {}",
                    rec.location,
                    rec.message,
                    p.display()
                ),
                Err(e) => tracing::error!(
                    "[CRASH-005] {program} stopped on an internal error ({}); its crash record couldn't be saved: {e}",
                    rec.message
                ),
            }
            default(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_found() {
        assert_eq!(code_in("[BINDLE-003] install failed"), Some("BINDLE-003"));
        assert_eq!(code_in("reelo[REELO-010] x"), Some("REELO-010"));
        assert_eq!(code_in("arr[3] [ab-123] [X-12]"), None);
    }

    #[test]
    fn crash_dir_follows_xdg() {
        let get = |k: &str| match k {
            "HOME" => Some("/home/a".to_string()),
            "XDG_STATE_HOME" => Some("relative".to_string()),
            _ => None,
        };
        assert_eq!(
            crash_dir_from(&get),
            PathBuf::from("/home/a/.local/state/uq/crashes")
        );
        let abs = |k: &str| (k == "XDG_STATE_HOME").then(|| "/s".to_string());
        assert_eq!(crash_dir_from(&abs), PathBuf::from("/s/uq/crashes"));
    }

    #[test]
    fn a_record_round_trips() {
        let dir = std::env::temp_dir().join(format!("uq-diag-test-{}", std::process::id()));
        let rec = PanicRecord {
            program: "demo".into(),
            version: "1".into(),
            time: 42,
            pid: 7,
            thread: "main".into(),
            message: "boom".into(),
            location: "src/main.rs:3".into(),
            backtrace: "bt".into(),
        };
        let folder = save_panic(&dir, &rec).unwrap();
        assert_eq!(folder, dir.join("demo-42-7"));
        let text = std::fs::read_to_string(folder.join("crash.json")).unwrap();
        let back = PanicRecord::from_json(&serde_json::from_str(&text).unwrap()).unwrap();
        assert_eq!(back, rec);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
