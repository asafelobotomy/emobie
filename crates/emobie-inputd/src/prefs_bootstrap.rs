//! Load expand settings from ~/.local/share/emobie/preferences.json when
//! no inputd-state.json exists yet (e.g. inputd starts at login before emobie).
//! An existing state file with empty matches is left alone (user cleared macros).

use crate::protocol::{MatchRule, TriggerMode};
use crate::state::PersistedState;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrefsMacro {
    trigger: String,
    expansion: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

fn preferences_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        })?;
    Some(base.join("emobie").join("preferences.json"))
}

fn trigger_mode_from_prefs(value: Option<&Value>) -> TriggerMode {
    match value.and_then(|v| v.as_str()).unwrap_or("space") {
        "immediate" => TriggerMode::Immediate,
        _ => TriggerMode::Space,
    }
}

fn matches_from_preferences(prefs: &Value) -> Option<Vec<MatchRule>> {
    let expand = prefs
        .get("expandAsYouType")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !expand {
        return None;
    }

    let mode = trigger_mode_from_prefs(prefs.get("expandTriggerMode"));
    let keep_space = prefs
        .get("expandKeepTriggerSpace")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let macros: Vec<PrefsMacro> = prefs
        .get("macros")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_default();

    let mut matches = Vec::new();
    for m in macros {
        if !m.enabled || m.trigger.is_empty() || m.expansion.is_empty() {
            continue;
        }
        let mut expansion = m.expansion;
        if mode == TriggerMode::Space && keep_space && !expansion.ends_with(' ') {
            expansion.push(' ');
        }
        matches.push(MatchRule {
            trigger: m.trigger,
            expansion,
            mode,
        });
    }

    if matches.is_empty() {
        return Some(Vec::new());
    }
    crate::state::validate_matches(&matches)
        .map_err(|err| eprintln!("warning: ignoring preferences bootstrap: {err}"))
        .ok()?;
    Some(matches)
}

/// When there is no on-disk state yet and matches are empty, mirror expand
/// settings from preferences.json. Returns true if `state` was updated.
/// Callers must only invoke this when `state::load` reported no state file.
pub fn apply_if_empty(state: &mut PersistedState) -> bool {
    if !state.matches.is_empty() {
        return false;
    }
    let Some(path) = preferences_path() else {
        return false;
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    if raw.len() > 2 * 1024 * 1024 {
        return false;
    }
    let Ok(root) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    let prefs = root.get("preferences").unwrap_or(&root);
    if !prefs.is_object() {
        return false;
    }
    let Some(matches) = matches_from_preferences(prefs) else {
        return false;
    };

    let expand = prefs
        .get("expandAsYouType")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    state.enabled = expand;
    state.matches = matches;
    if expand {
        eprintln!(
            "emobie-inputd: bootstrapped {} match(es) from preferences.json",
            state.matches.len()
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::TriggerMode;

    #[test]
    fn builds_space_mode_matches_with_trailing_space() {
        let prefs = serde_json::json!({
            "expandAsYouType": true,
            "expandTriggerMode": "space",
            "expandKeepTriggerSpace": true,
            "macros": [{
                "trigger": ".links",
                "expansion": "hello",
                "enabled": true
            }]
        });
        let matches = matches_from_preferences(&prefs).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].trigger, ".links");
        assert_eq!(matches[0].expansion, "hello ");
        assert_eq!(matches[0].mode, TriggerMode::Space);
    }

    #[test]
    fn skips_when_expand_disabled_in_prefs() {
        let prefs = serde_json::json!({
            "expandAsYouType": false,
            "macros": [{ "trigger": "x", "expansion": "y", "enabled": true }]
        });
        assert!(matches_from_preferences(&prefs).is_none());
    }
}
