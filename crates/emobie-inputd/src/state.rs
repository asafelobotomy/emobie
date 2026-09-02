//! Persist expand enablement + matches across daemon restarts / reboot.

use crate::protocol::{MatchRule, TriggerMode};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static SAVE_MUTEX: Mutex<()> = Mutex::new(());

pub const MAX_MATCHES: usize = 2_000;
pub const MAX_TRIGGER_LEN: usize = 256;
pub const MAX_EXPANSION_LEN: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub matches: Vec<MatchRule>,
}

fn state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        })?;
    Some(base.join("emobie").join("inputd-state.json"))
}

pub fn validate_matches(matches: &[MatchRule]) -> Result<(), String> {
    if matches.len() > MAX_MATCHES {
        return Err(format!("too many matches (max {MAX_MATCHES})"));
    }
    for m in matches {
        if m.trigger.is_empty() {
            return Err("trigger must not be empty".into());
        }
        if m.expansion.is_empty() {
            return Err("expansion must not be empty".into());
        }
        if m.trigger.contains('\0') || m.expansion.contains('\0') {
            return Err("trigger/expansion must not contain NUL".into());
        }
        // Triggers are built from printable key events — reject control chars so
        // a bad sync cannot install rules that can never fire (or confuse erase).
        if m.trigger.chars().any(|c| c.is_control()) {
            return Err("trigger must not contain control characters".into());
        }
        if m.trigger.chars().count() > MAX_TRIGGER_LEN {
            return Err(format!(
                "trigger longer than {MAX_TRIGGER_LEN} characters"
            ));
        }
        if m.expansion.len() > MAX_EXPANSION_LEN {
            return Err(format!(
                "expansion longer than {MAX_EXPANSION_LEN} bytes"
            ));
        }
    }
    Ok(())
}

/// Keep the last rule for each trigger (trie insert would overwrite anyway).
pub fn dedupe_matches(matches: Vec<MatchRule>) -> Vec<MatchRule> {
    use std::collections::HashMap;
    let mut last_at: HashMap<String, usize> = HashMap::new();
    for (i, m) in matches.iter().enumerate() {
        last_at.insert(m.trigger.clone(), i);
    }
    matches
        .into_iter()
        .enumerate()
        .filter(|(i, m)| last_at.get(&m.trigger) == Some(i))
        .map(|(_, m)| m)
        .collect()
}

/// Drop matches that exceed caps so a corrupt/huge file cannot blow the trie.
fn sanitize_matches(matches: Vec<MatchRule>) -> Vec<MatchRule> {
    let cleaned: Vec<MatchRule> = matches
        .into_iter()
        .filter(|m| {
            !m.trigger.is_empty()
                && !m.expansion.is_empty()
                && !m.trigger.contains('\0')
                && !m.expansion.contains('\0')
                && !m.trigger.chars().any(|c| c.is_control())
                && m.trigger.chars().count() <= MAX_TRIGGER_LEN
                && m.expansion.len() <= MAX_EXPANSION_LEN
        })
        .take(MAX_MATCHES)
        .collect();
    dedupe_matches(cleaned)
}

/// Load persisted state. Second value is true when a state file was read from disk
/// (even if matches are empty) — empty on-disk matches must not re-bootstrap prefs.
pub fn load() -> (PersistedState, bool) {
    let Some(path) = state_path() else {
        return (PersistedState::default(), false);
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return (PersistedState::default(), false);
    };
    // Cap raw size roughly to one request budget.
    if raw.len() > 512 * 1024 {
        eprintln!(
            "warning: ignoring oversized state file {} ({} bytes)",
            path.display(),
            raw.len()
        );
        return (PersistedState::default(), false);
    }
    let mut state: PersistedState = serde_json::from_str(&raw).unwrap_or_default();
    state.matches = sanitize_matches(state.matches);
    (state, true)
}

pub fn save(enabled: bool, matches: &[MatchRule]) {
    let Ok(_guard) = SAVE_MUTEX.lock() else {
        return;
    };
    save_locked(enabled, matches);
}

/// Persist under `SAVE_MUTEX`, re-reading `enabled` after the lock so a concurrent
/// SetEnabled cannot be overwritten by a stale SyncMatches snapshot.
pub fn save_reloading_enabled(enabled: &AtomicBool, matches: &[MatchRule]) {
    let Ok(_guard) = SAVE_MUTEX.lock() else {
        return;
    };
    save_locked(enabled.load(Ordering::Relaxed), matches);
}

fn save_locked(enabled: bool, matches: &[MatchRule]) {
    let Some(path) = state_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("warning: could not create state dir: {err}");
            return;
        }
        if let Ok(meta) = fs::metadata(parent) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(parent, perms);
        }
    }
    let state = PersistedState {
        enabled,
        matches: matches.to_vec(),
    };
    let Ok(body) = serde_json::to_string(&state) else {
        return;
    };
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = path.with_extension(format!("json.{nonce}.tmp"));
    let write_result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
        // Ensure final mode even if umask was loose.
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        let _ = fs::set_permissions(&tmp, perms);
        fs::rename(&tmp, &path)?;
        Ok(())
    })();
    if let Err(err) = write_result {
        eprintln!("warning: failed to save inputd state: {err}");
        let _ = fs::remove_file(&tmp);
    }
}

pub fn pairs_from_matches(matches: &[MatchRule]) -> Vec<(String, String, TriggerMode)> {
    matches
        .iter()
        .map(|m| (m.trigger.clone(), m.expansion.clone(), m.mode))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{sanitize_matches, validate_matches, MAX_TRIGGER_LEN};
    use crate::protocol::{MatchRule, TriggerMode};

    fn rule(trigger: &str, expansion: &str) -> MatchRule {
        MatchRule {
            trigger: trigger.to_string(),
            expansion: expansion.to_string(),
            mode: TriggerMode::Immediate,
        }
    }

    #[test]
    fn validate_rejects_oversized_trigger() {
        let big = "a".repeat(MAX_TRIGGER_LEN + 1);
        assert!(validate_matches(&[rule(&big, "x")]).is_err());
    }

    #[test]
    fn sanitize_drops_bad_matches() {
        let big = "a".repeat(MAX_TRIGGER_LEN + 1);
        let kept = sanitize_matches(vec![rule("ok", "yes"), rule(&big, "no")]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].trigger, "ok");
    }

    #[test]
    fn validate_rejects_nul_and_controls() {
        assert!(validate_matches(&[rule("a\0b", "x")]).is_err());
        assert!(validate_matches(&[rule("ab", "x\0")]).is_err());
        assert!(validate_matches(&[rule("a\nb", "x")]).is_err());
    }

    #[test]
    fn dedupe_keeps_last_trigger() {
        use super::dedupe_matches;
        let out = dedupe_matches(vec![
            rule(":a", "one"),
            rule(":b", "bee"),
            rule(":a", "two"),
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].trigger, ":b");
        assert_eq!(out[1].trigger, ":a");
        assert_eq!(out[1].expansion, "two");
    }
}
