//! The settings `emobie config` knows: each one's type, default and what the app accepts for
//! it, mirroring `src/lib/normalizePreferences.ts` and `src/types/preferences.ts`.

use super::store::Prefs;
use serde_json::{json, Value};

pub enum Kind {
    Bool,
    Enum(&'static [&'static str]),
    Int { min: i64, max: i64 },
    Hotkey,
    /// A list of app classes (strings), edited with `config add|remove`.
    Apps,
}

pub struct Setting {
    pub key: &'static str,
    pub kind: Kind,
    /// The JSON default, as `DEFAULT_PREFERENCES` has it.
    pub default: fn() -> Value,
    /// Why `config set` refuses it: these need a permission only the app can ask for.
    pub readonly: Option<&'static str>,
}

const NEEDS_APP: &str = "Needs a one-time permission, so it is turned on in emobie itself.";

/// Password managers and authentication prompts (`DEFAULT_EXCLUDED_APPS`).
pub const DEFAULT_EXCLUDED_APPS: [&str; 8] = [
    "keepassxc",
    "bitwarden",
    "1password",
    "org.gnome.seahorse",
    "pinentry",
    "gcr-prompter",
    "polkit",
    "ksshaskpass",
];
/// Keep in sync with emobie-inputd `rpc.rs` MAX_EXCLUDED_APPS / MAX_EXCLUDED_APP_LEN.
pub const MAX_EXCLUDED_APPS: usize = 64;
pub const MAX_EXCLUDED_APP_LEN: usize = 128;

macro_rules! s {
    ($key:literal, $kind:expr, $default:expr) => {
        Setting { key: $key, kind: $kind, default: || json!($default), readonly: None }
    };
}

pub fn all() -> Vec<Setting> {
    use Kind::*;
    vec![
        s!("theme", Enum(&["system", "light", "dark"]), "system"),
        s!("emojiSize", Enum(&["sm", "md", "lg"]), "md"),
        s!("sortBy", Enum(&["default", "name", "type", "dateAdded", "uses"]), "default"),
        s!("skinTone", Int { min: 0, max: 5 }, 0),
        s!("recentMax", Int { min: 8, max: 96 }, 32),
        s!("hotkey", Hotkey, "Control+Shift+Space"),
        s!("showTitleBar", Bool, false),
        s!("launchOnStartup", Bool, false),
        s!("startMinimizedToTray", Bool, false),
        s!("allowMultipleInstances", Bool, false),
        s!("checkUpdatesOnStartup", Bool, true),
        s!("favoriteEmojiMacros", Bool, false),
        s!("emoticonStyle", Enum(&["minimal", "classic"]), "minimal"),
        s!("expandTriggerMode", Enum(&["immediate", "space"]), "space"),
        s!("expandKeepTriggerSpace", Bool, false),
        s!("expandRestoreClipboard", Bool, false),
        s!(
            "pasteChordOverride",
            Enum(&["auto", "ctrl_v", "shift_insert", "ctrl_shift_v"]),
            "auto"
        ),
        s!("expandExcludedApps", Apps, DEFAULT_EXCLUDED_APPS),
        Setting { readonly: Some(NEEDS_APP), ..s!("autoPasteOnCopy", Bool, false) },
        Setting { readonly: Some(NEEDS_APP), ..s!("expandAsYouType", Bool, false) },
    ]
}

pub fn find(key: &str) -> Option<Setting> {
    all().into_iter().find(|s| s.key == key)
}

/// A hotkey as the app stores it: modifiers and one key joined with `+` (`Control+Shift+Space`).
/// Like the app, a letter or digit needs Control, Alt or Meta.
pub fn valid_hotkey(raw: &str) -> bool {
    let parts: Vec<&str> = raw.split('+').collect();
    if parts.len() < 2 || parts.len() > 5 {
        return false;
    }
    let (key, mods) = parts.split_last().unwrap();
    let modifier = |m: &&str| matches!(*m, "Control" | "Shift" | "Alt" | "Meta" | "Super");
    if !mods.iter().all(modifier) {
        return false;
    }
    let mut seen = std::collections::HashSet::new();
    if !mods.iter().all(|m| seen.insert(*m)) {
        return false;
    }
    let named = matches!(
        *key,
        "Space" | "Up" | "Down" | "Left" | "Right" | "Esc" | "Enter" | "Tab" | "Backspace"
            | "Delete" | "Home" | "End" | "PageUp" | "PageDown" | "Insert"
    ) || (key.len() >= 2
        && key.starts_with('F')
        && key[1..].parse::<u8>().is_ok_and(|n| (1..=24).contains(&n)));
    let single = key.chars().count() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
    if single {
        return mods.iter().any(|m| matches!(*m, "Control" | "Alt" | "Meta"));
    }
    named
}

fn to_bool(raw: &str) -> Option<bool> {
    match raw.to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" | "1" => Some(true),
        "false" | "off" | "no" | "0" => Some(false),
        _ => None,
    }
}

/// Parses `raw` for `setting`, or says what is wrong with it.
pub fn parse(setting: &Setting, raw: &str) -> Result<Value, String> {
    match &setting.kind {
        Kind::Bool => to_bool(raw)
            .map(Value::Bool)
            .ok_or_else(|| format!("{} is on or off, not \"{raw}\"", setting.key)),
        Kind::Enum(allowed) => {
            if allowed.contains(&raw) {
                Ok(json!(raw))
            } else {
                Err(format!("{} is one of {}, not \"{raw}\"", setting.key, allowed.join(", ")))
            }
        }
        Kind::Int { min, max } => match raw.parse::<i64>() {
            Ok(n) if (*min..=*max).contains(&n) => Ok(json!(n)),
            _ => Err(format!("{} is a whole number from {min} to {max}, not \"{raw}\"", setting.key)),
        },
        Kind::Hotkey => {
            if valid_hotkey(raw) {
                Ok(json!(raw))
            } else {
                Err(format!(
                    "\"{raw}\" is not a hotkey: use modifiers and a key, like Control+Shift+Space \
                     (a letter or digit needs Control, Alt or Meta)"
                ))
            }
        }
        Kind::Apps => Err(format!("{} is a list: use config add or config remove", setting.key)),
    }
}

/// Whether `value` is one the app would keep for this setting (else it falls back to the default).
fn is_valid(setting: &Setting, value: &Value) -> bool {
    match (&setting.kind, value) {
        (Kind::Bool, Value::Bool(_)) => true,
        (Kind::Enum(allowed), Value::String(s)) => allowed.contains(&s.as_str()),
        (Kind::Int { min, max }, v) => v.as_f64().is_some_and(|n| n >= *min as f64 && n <= *max as f64),
        (Kind::Hotkey, Value::String(s)) => !s.trim().is_empty(),
        (Kind::Apps, Value::Array(_)) => true,
        _ => false,
    }
}

/// What the app currently uses for `setting`: the saved value, or the default when none or
/// an unusable one is saved.
pub fn current(setting: &Setting, prefs: &Prefs) -> Value {
    match prefs.get(setting.key) {
        Some(v) if is_valid(setting, v) => match (&setting.kind, v) {
            (Kind::Int { .. }, v) => json!(v.as_f64().unwrap_or(0.0).round() as i64),
            (Kind::Hotkey, Value::String(s)) => json!(s.trim()),
            (Kind::Apps, v) => json!(clean_apps(v)),
            _ => v.clone(),
        },
        _ => (setting.default)(),
    }
}

/// App classes as the app keeps them: trimmed, no empties, no case-insensitive duplicates, capped.
pub fn clean_apps(value: &Value) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in value.as_array().into_iter().flatten() {
        let Some(app) = item.as_str().map(str::trim) else { continue };
        if app.is_empty() || app.chars().count() > MAX_EXCLUDED_APP_LEN || !seen.insert(app.to_lowercase()) {
            continue;
        }
        out.push(app.to_string());
        if out.len() >= MAX_EXCLUDED_APPS {
            break;
        }
    }
    out
}

/// One setting as `config show --json` prints it.
pub fn describe(setting: &Setting, prefs: &Prefs) -> Value {
    let mut out = json!({
        "value": current(setting, prefs),
        "default": (setting.default)(),
        "saved": prefs.contains_key(setting.key),
        "readonly": setting.readonly.is_some(),
    });
    let o = out.as_object_mut().unwrap();
    match &setting.kind {
        Kind::Bool => {
            o.insert("type".into(), json!("bool"));
        }
        Kind::Enum(allowed) => {
            o.insert("type".into(), json!("enum"));
            o.insert("allowed".into(), json!(allowed));
        }
        Kind::Int { min, max } => {
            o.insert("type".into(), json!("int"));
            o.insert("min".into(), json!(min));
            o.insert("max".into(), json!(max));
        }
        Kind::Hotkey => {
            o.insert("type".into(), json!("hotkey"));
        }
        Kind::Apps => {
            o.insert("type".into(), json!("apps"));
            o.insert("max".into(), json!(MAX_EXCLUDED_APPS));
        }
    }
    if let Some(reason) = setting.readonly {
        o.insert("reason".into(), json!(reason));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_follow_the_apps_rule() {
        assert!(valid_hotkey("Control+Shift+Space"));
        assert!(valid_hotkey("Alt+F4"));
        assert!(valid_hotkey("Control+E"));
        assert!(!valid_hotkey("Shift+E"), "a letter needs Control, Alt or Meta");
        assert!(!valid_hotkey("E"));
        assert!(!valid_hotkey("Control+Control+E"));
        assert!(!valid_hotkey("Ctrl+E"));
        assert!(!valid_hotkey("Control+"));
        assert!(!valid_hotkey("Control+Shift+Wat"));
    }

    #[test]
    fn parse_checks_every_kind() {
        let theme = find("theme").unwrap();
        assert_eq!(parse(&theme, "dark").unwrap(), json!("dark"));
        assert!(parse(&theme, "blue").is_err());
        let size = find("recentMax").unwrap();
        assert_eq!(parse(&size, "40").unwrap(), json!(40));
        assert!(parse(&size, "7").is_err());
        assert!(parse(&size, "97").is_err());
        assert!(parse(&size, "x").is_err());
        let title = find("showTitleBar").unwrap();
        assert_eq!(parse(&title, "on").unwrap(), json!(true));
        assert!(parse(&title, "maybe").is_err());
        assert!(parse(&find("expandExcludedApps").unwrap(), "a").is_err());
    }

    #[test]
    fn current_falls_back_to_the_default_for_an_unusable_value() {
        let mut prefs = Prefs::new();
        let theme = find("theme").unwrap();
        assert_eq!(current(&theme, &prefs), json!("system"));
        prefs.insert("theme".into(), json!("blue"));
        assert_eq!(current(&theme, &prefs), json!("system"));
        prefs.insert("theme".into(), json!("light"));
        assert_eq!(current(&theme, &prefs), json!("light"));
        let apps = find("expandExcludedApps").unwrap();
        prefs.insert("expandExcludedApps".into(), json!(["A", "a", " ", "B"]));
        assert_eq!(current(&apps, &prefs), json!(["A", "B"]));
    }

    #[test]
    fn the_two_permission_settings_are_read_only() {
        for key in ["autoPasteOnCopy", "expandAsYouType"] {
            assert!(find(key).unwrap().readonly.is_some());
        }
        assert!(find("theme").unwrap().readonly.is_none());
    }
}
