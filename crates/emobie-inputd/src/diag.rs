//! Problems in uQ's log format (uQ's `native/uq-diag`, vendored as `uq_diag.rs`). Each warning
//! and error starts with its code from `uq/diag/emobie.toml`, `[EMOBIE-002] …`. Under systemd the
//! line starts with its syslog level, `<4>emobie-inputd: [EMOBIE-002] …`, so the journal files
//! it as a warning and uQ's Settings › Problems lists it.
//!
//! No panic hook here: the helper catches the panics its input backends raise and carries on.

pub use tracing::Level;

use crate::uq_diag::{priority, Style};

/// The line for `msg`: with its level under the journal, plain otherwise.
pub fn line(level: Level, msg: &str, journal: bool) -> String {
    if journal {
        format!("<{}>emobie-inputd: {msg}", priority(&level))
    } else {
        format!("emobie-inputd: {msg}")
    }
}

/// Write a problem to stderr.
pub fn problem(level: Level, msg: &str) {
    eprintln!("{}", line(level, msg, Style::detect() == Style::Journal));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_lines_carry_their_level() {
        assert_eq!(
            line(Level::WARN, "[EMOBIE-002] x", true),
            "<4>emobie-inputd: [EMOBIE-002] x"
        );
        assert_eq!(
            line(Level::ERROR, "[EMOBIE-001] x", false),
            "emobie-inputd: [EMOBIE-001] x"
        );
    }
}
