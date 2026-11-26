//! Macros and the recently deleted ones, edited in the preferences the way the app does it
//! (`src/lib/normalizePreferences.ts`, `src/hooks/usePreferences.ts`).

use super::plan::{Change, Plan};
use super::settings;
use super::store::Prefs;
use serde_json::{json, Value};

/// Deleted macros kept for restoring, newest first. Keep in sync with `DELETED_MACROS_MAX` in
/// `src/lib/normalizePreferences.ts`.
pub const DELETED_MAX: usize = 50;
/// Keep in sync with emobie-inputd `state.rs`.
const MAX_MACROS: usize = 2_000;
const MAX_TRIGGER_CHARS: usize = 256;
const MAX_EXPANSION_BYTES: usize = 64 * 1024;

pub enum Op {
    Add { trigger: String, expansion: String, hotkey: Option<String> },
    Set { id: String, enabled: Option<bool>, trigger: Option<String>, expansion: Option<String>, hotkey: Option<String> },
    Remove { id: String },
    Restore { id: String },
    /// One deleted macro, or all of them (`id` None).
    Purge { id: Option<String> },
}

fn has_control(s: &str) -> bool {
    s.chars().any(|c| c <= '\u{1f}' || ('\u{7f}'..='\u{9f}').contains(&c))
}

fn text(m: &Value, key: &str) -> String {
    m.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

/// A macro the app would keep: an id, a trigger, an expansion, and text the input helper accepts.
fn valid(m: &Value) -> bool {
    let (id, trigger, expansion) = (text(m, "id"), text(m, "trigger"), text(m, "expansion"));
    !id.trim().is_empty()
        && !trigger.trim().is_empty()
        && !expansion.is_empty()
        && !has_control(trigger.trim())
        && !expansion.contains('\0')
}

/// `unique_triggers`: live macros can't share a trigger; deleted ones can (restoring checks).
fn list_of(prefs: &Prefs, key: &str, unique_triggers: bool) -> Vec<Value> {
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_triggers = std::collections::HashSet::new();
    prefs
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|m| m.is_object() && valid(m))
        .filter(|m| seen_ids.insert(text(m, "id")) && (!unique_triggers || seen_triggers.insert(text(m, "trigger").trim().to_string())))
        .cloned()
        .collect()
}

pub fn macros(prefs: &Prefs) -> Vec<Value> {
    list_of(prefs, "macros", true)
}

/// Newest deletion first.
pub fn deleted(prefs: &Prefs) -> Vec<Value> {
    let mut items: Vec<Value> = list_of(prefs, "deletedMacros", false);
    items.sort_by_key(|m| std::cmp::Reverse(m.get("deletedAt").and_then(Value::as_u64).unwrap_or(0)));
    items.truncate(DELETED_MAX);
    items
}

/// One macro as the commands print it.
fn view(m: &Value) -> Value {
    let mut out = json!({
        "id": text(m, "id"),
        "trigger": text(m, "trigger").trim(),
        "expansion": text(m, "expansion"),
        "hotkey": m.get("hotkey").and_then(Value::as_str).map(str::trim).filter(|h| !h.is_empty()),
        "enabled": m.get("enabled") != Some(&Value::Bool(false)),
        "font_style": m.get("fontStyle").and_then(Value::as_str),
    });
    if let Some(at) = m.get("deletedAt").and_then(Value::as_u64) {
        out["deleted_at"] = json!(at);
    }
    out
}

pub fn macro_views(prefs: &Prefs) -> Vec<Value> {
    macros(prefs).iter().map(view).collect()
}

pub fn deleted_views(prefs: &Prefs) -> Vec<Value> {
    deleted(prefs).iter().map(view).collect()
}

/// Finds a macro by id, else by its exact trigger.
fn position(list: &[Value], key: &str) -> Option<usize> {
    list.iter()
        .position(|m| text(m, "id") == key)
        .or_else(|| list.iter().position(|m| text(m, "trigger").trim() == key))
}

fn check_text(trigger: &str, expansion: &str) -> Result<(), String> {
    if trigger.is_empty() {
        return Err("A macro needs a trigger.".into());
    }
    if has_control(trigger) {
        return Err("A trigger can't contain control characters.".into());
    }
    if trigger.chars().count() > MAX_TRIGGER_CHARS {
        return Err(format!("A trigger is at most {MAX_TRIGGER_CHARS} characters."));
    }
    if expansion.is_empty() {
        return Err("A macro needs an expansion.".into());
    }
    if expansion.contains('\0') {
        return Err("An expansion can't contain a NUL character.".into());
    }
    if expansion.len() > MAX_EXPANSION_BYTES {
        return Err(format!("An expansion is at most {} KiB.", MAX_EXPANSION_BYTES / 1024));
    }
    Ok(())
}

fn check_hotkey(prefs: &Prefs, list: &[Value], hotkey: &str, skip_id: &str) -> Result<(), String> {
    if !settings::valid_hotkey(hotkey) {
        return Err(format!("\"{hotkey}\" is not a hotkey, like Control+Alt+S."));
    }
    let summon = settings::current(&settings::find("hotkey").unwrap(), prefs);
    if summon.as_str() == Some(hotkey) {
        return Err("That hotkey is the summon shortcut.".into());
    }
    let clash = list.iter().find(|m| {
        text(m, "id") != skip_id
            && m.get("enabled") != Some(&Value::Bool(false))
            && m.get("hotkey").and_then(Value::as_str) == Some(hotkey)
    });
    match clash {
        Some(m) => Err(format!("That hotkey is already used by {}.", text(m, "trigger").trim())),
        None => Ok(()),
    }
}

fn new_id() -> String {
    // A random (v4) UUID, as the app makes with crypto.randomUUID().
    let mut b = [0u8; 16];
    let ok = std::fs::File::open("/dev/urandom").and_then(|mut f| std::io::Read::read_exact(&mut f, &mut b));
    if ok.is_err() {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        b = (n ^ (std::process::id() as u128) << 64).to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

fn put(prefs: &mut Prefs, key: &str, list: Vec<Value>) {
    prefs.insert(key.into(), Value::Array(list));
}

fn shown(m: &Value) -> String {
    text(m, "trigger").trim().to_string()
}

fn expansion_detail(m: &Value) -> String {
    let e = text(m, "expansion");
    let one: String = e.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if one.chars().count() > 80 {
        format!("{}…", one.chars().take(80).collect::<String>())
    } else {
        one
    }
}

/// What `op` would do, without doing it.
pub fn plan(prefs: &Prefs, op: &Op) -> Plan {
    match op {
        Op::Remove { id } => {
            let list = macros(prefs);
            let Some(i) = position(&list, id) else { return Plan::refuse("There is no such macro.") };
            Plan::new(
                "1 macro moves to Deleted",
                vec![Change {
                    group: "Macros",
                    label: shown(&list[i]),
                    detail: format!("{} — can be restored", expansion_detail(&list[i])),
                    reversible: true,
                }],
            )
        }
        Op::Purge { id } => {
            let all = deleted(prefs);
            let picked: Vec<&Value> = match id {
                Some(key) => match position(&all, key) {
                    Some(i) => vec![&all[i]],
                    None => return Plan::refuse("There is no such deleted macro."),
                },
                None => all.iter().collect(),
            };
            if picked.is_empty() {
                return Plan::refuse("Nothing is deleted.");
            }
            Plan::new(
                format!("{} deleted macro{} removed for good", picked.len(), if picked.len() == 1 { "" } else { "s" }),
                picked
                    .iter()
                    .map(|m| Change {
                        group: "Deleted macros",
                        label: shown(m),
                        detail: expansion_detail(m),
                        reversible: false,
                    })
                    .collect(),
            )
        }
        other => {
            let mut scratch = prefs.clone();
            match apply(&mut scratch, other, 0) {
                Ok(v) => Plan::new(
                    v["summary"].as_str().unwrap_or("Saved").to_string(),
                    vec![Change {
                        group: "Macros",
                        label: v["macro"]["trigger"].as_str().unwrap_or("").to_string(),
                        detail: v["summary"].as_str().unwrap_or("").to_string(),
                        reversible: true,
                    }],
                ),
                Err(e) => Plan::refuse(e),
            }
        }
    }
}

/// Does `op` on `prefs` and says what happened as `{summary, macro?}`.
pub fn apply(prefs: &mut Prefs, op: &Op, now_ms: u64) -> Result<Value, String> {
    match op {
        Op::Add { trigger, expansion, hotkey } => {
            let trigger = trigger.trim();
            check_text(trigger, expansion)?;
            let mut list = macros(prefs);
            if list.len() >= MAX_MACROS {
                return Err(format!("There can be at most {MAX_MACROS} macros."));
            }
            if list.iter().any(|m| text(m, "trigger").trim() == trigger) {
                return Err("Another macro already uses this trigger.".into());
            }
            let hotkey = hotkey.as_deref().map(str::trim).filter(|h| !h.is_empty());
            if let Some(h) = hotkey {
                check_hotkey(prefs, &list, h, "")?;
            }
            let m = json!({
                "id": new_id(), "trigger": trigger, "expansion": expansion,
                "hotkey": hotkey, "enabled": true,
            });
            list.push(m.clone());
            put(prefs, "macros", list);
            Ok(json!({"summary": format!("Added {trigger}"), "macro": view(&m)}))
        }
        Op::Set { id, enabled, trigger, expansion, hotkey } => {
            let mut list = macros(prefs);
            let i = position(&list, id).ok_or("There is no such macro.")?;
            let mut m = list[i].clone();
            let own_id = text(&m, "id");
            let trigger_new = trigger.as_deref().map(|t| t.trim().to_string()).unwrap_or_else(|| shown(&m));
            let expansion_new = expansion.clone().unwrap_or_else(|| text(&m, "expansion"));
            check_text(&trigger_new, &expansion_new)?;
            if list.iter().any(|o| text(o, "id") != own_id && text(o, "trigger").trim() == trigger_new) {
                return Err("Another macro already uses this trigger.".into());
            }
            let obj = m.as_object_mut().unwrap();
            obj.insert("trigger".into(), json!(trigger_new));
            obj.insert("expansion".into(), json!(expansion_new));
            if let Some(on) = enabled {
                obj.insert("enabled".into(), json!(on));
            }
            if let Some(h) = hotkey {
                let h = h.trim();
                obj.insert("hotkey".into(), if h.is_empty() { Value::Null } else { json!(h) });
            }
            // A hotkey is checked after every change that could make it collide: a new one, or
            // switching the macro back on.
            let now_hotkey = obj.get("hotkey").and_then(Value::as_str).map(String::from);
            let now_enabled = obj.get("enabled") != Some(&Value::Bool(false));
            if let (Some(h), true) = (now_hotkey, now_enabled) {
                if hotkey.is_some() || *enabled == Some(true) {
                    check_hotkey(prefs, &list, &h, &own_id)?;
                }
            }
            let summary = match (enabled, trigger.is_some() || expansion.is_some() || hotkey.is_some()) {
                (Some(true), false) => format!("Turned on {trigger_new}"),
                (Some(false), false) => format!("Turned off {trigger_new}"),
                _ => format!("Saved {trigger_new}"),
            };
            list[i] = m.clone();
            put(prefs, "macros", list);
            Ok(json!({"summary": summary, "macro": view(&m)}))
        }
        Op::Remove { id } => {
            let mut list = macros(prefs);
            let i = position(&list, id).ok_or("There is no such macro.")?;
            let mut gone = list.remove(i);
            gone.as_object_mut().unwrap().insert("deletedAt".into(), json!(now_ms));
            let mut bin: Vec<Value> = deleted(prefs)
                .into_iter()
                .filter(|m| text(m, "id") != text(&gone, "id"))
                .collect();
            bin.insert(0, gone.clone());
            bin.truncate(DELETED_MAX);
            put(prefs, "macros", list);
            put(prefs, "deletedMacros", bin);
            Ok(json!({"summary": format!("Deleted {}", shown(&gone)), "macro": view(&gone)}))
        }
        Op::Restore { id } => {
            let mut bin = deleted(prefs);
            let i = position(&bin, id).ok_or("There is no such deleted macro.")?;
            let mut list = macros(prefs);
            let mut back = bin[i].clone();
            if list.iter().any(|m| text(m, "trigger").trim() == shown(&back)) {
                return Err(format!("Another macro already uses {}.", shown(&back)));
            }
            if list.len() >= MAX_MACROS {
                return Err(format!("There can be at most {MAX_MACROS} macros."));
            }
            bin.remove(i);
            let obj = back.as_object_mut().unwrap();
            obj.remove("deletedAt");
            // The hotkey may have been taken since: bring the macro back without it.
            let mut note = "";
            let own_id = text(&Value::Object(obj.clone()), "id");
            if let Some(h) = obj.get("hotkey").and_then(Value::as_str).map(String::from) {
                if check_hotkey(prefs, &list, &h, &own_id).is_err() {
                    obj.insert("hotkey".into(), Value::Null);
                    note = " (its hotkey is taken, so it has none)";
                }
            }
            list.push(back.clone());
            put(prefs, "macros", list);
            put(prefs, "deletedMacros", bin);
            Ok(json!({"summary": format!("Restored {}{note}", shown(&back)), "macro": view(&back)}))
        }
        Op::Purge { id } => {
            let bin = deleted(prefs);
            let (keep, n) = match id {
                Some(key) => {
                    let i = position(&bin, key).ok_or("There is no such deleted macro.")?;
                    let mut keep = bin.clone();
                    keep.remove(i);
                    (keep, 1)
                }
                None => (Vec::new(), bin.len()),
            };
            put(prefs, "deletedMacros", keep);
            Ok(json!({"summary": format!("Removed {n} deleted macro{} for good", if n == 1 { "" } else { "s" }), "removed": n}))
        }
    }
}

#[cfg(test)]
#[path = "macros_tests.rs"]
mod tests;
