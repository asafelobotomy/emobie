//! `emobie status | config | macro`: the app's settings and macros from the command line, for
//! scripts and for the uQ Settings page (`uq/pages/emobie`). They read and write the same
//! preferences file the app uses, and a running app picks the change up by itself.
//!
//! `run` is called before the window starts, so these never open one. Every command takes
//! `--json`; the ones that change something take `--dry-run`, which prints the plan uQ's
//! confirmation sheet shows. With `--json` a failure is still a JSON document
//! (`{"ok": false, "error": …}`) and the exit code is 1.

mod macros;
mod plan;
mod settings;
mod store;
#[cfg(test)]
mod testenv;

use macros::Op;
use serde_json::{json, Value};
use settings::Kind;
use std::collections::HashMap;
use store::Prefs;

const USAGE: &str = "\
Usage: emobie <command>

  status [--json]                           what is saved, settings and macros
  config show [--json]                      every setting with its value and default
  config set <key> <value> [--dry-run]      change a setting
  config add|remove <key> <value>           edit a list setting (expandExcludedApps)
  config reset <key> [--dry-run]            back to the default
  macro list [--json]                       your macros
  macro add <trigger> <expansion> [--hotkey <keys>] [--dry-run]
  macro set <id|trigger> [--enabled true|false] [--trigger <t>] [--expansion <e>] [--hotkey <keys>]
  macro remove <id|trigger> [--dry-run]     moves it to Deleted (it can be restored)
  macro deleted [--json]                    recently deleted macros
  macro restore <id|trigger>                brings a deleted macro back
  macro purge <id|trigger> | --all [--dry-run]   removes deleted macros for good

Every command takes --json. The window starts with no command.";

/// The commands this module answers; anything else starts the app as before.
const COMMANDS: [&str; 3] = ["status", "config", "macro"];

struct Args {
    words: Vec<String>,
    flags: HashMap<String, String>,
    json: bool,
    dry_run: bool,
}

const VALUE_FLAGS: [&str; 4] = ["enabled", "trigger", "expansion", "hotkey"];

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut out = Args { words: vec![], flags: HashMap::new(), json: false, dry_run: false };
    let mut it = args.iter();
    let mut only_words = false;
    while let Some(arg) = it.next() {
        if only_words || !arg.starts_with("--") {
            out.words.push(arg.clone());
            continue;
        }
        if arg == "--" {
            only_words = true;
            continue;
        }
        let name = &arg[2..];
        match name {
            "json" => out.json = true,
            "dry-run" => out.dry_run = true,
            "all" => {
                out.flags.insert("all".into(), "1".into());
            }
            n if VALUE_FLAGS.contains(&n) => {
                let v = it.next().ok_or_else(|| format!("--{n} needs a value"))?;
                out.flags.insert(n.into(), v.clone());
            }
            _ => return Err(format!("unknown option {arg}")),
        }
    }
    Ok(out)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// What a command ends with: a JSON document and its plain-text form.
struct Output {
    json: Value,
    text: String,
    ok: bool,
}

impl Output {
    fn ok(json: Value, text: impl Into<String>) -> Self {
        Self { json, text: text.into(), ok: true }
    }
    fn fail(error: impl Into<String>) -> Self {
        let error = error.into();
        Self { json: json!({"ok": false, "error": error}), text: error, ok: false }
    }
}

fn config_view(prefs: &Prefs) -> Value {
    let mut map = serde_json::Map::new();
    for s in settings::all() {
        map.insert(s.key.into(), settings::describe(&s, prefs));
    }
    Value::Object(map)
}

fn status(loaded: &store::Loaded) -> Output {
    let p = &loaded.prefs;
    let json = json!({
        "app": {
            "version": env!("CARGO_PKG_VERSION"),
            "saved": loaded.path.is_some(),
            "path": loaded.path.as_ref().map(|p| p.display().to_string()),
            "deleted_max": macros::DELETED_MAX,
        },
        "config": config_view(p),
        "macros": macros::macro_views(p),
        "deleted": macros::deleted_views(p),
    });
    let text = format!(
        "emobie {}\n{} macro(s), {} deleted\n{}",
        env!("CARGO_PKG_VERSION"),
        macros::macros(p).len(),
        macros::deleted(p).len(),
        match &loaded.path {
            Some(path) => format!("preferences: {}", path.display()),
            None => "no preferences saved yet: open emobie once".into(),
        }
    );
    Output::ok(json, text)
}

/// Runs `change` on the saved preferences and saves the result, or just plans it.
fn mutate(
    args: &Args,
    plan: impl Fn(&Prefs) -> plan::Plan,
    change: impl Fn(&mut Prefs) -> Result<Value, String>,
) -> Output {
    let loaded = match store::load() {
        Ok(l) => l,
        Err(e) => return Output::fail(e),
    };
    if args.dry_run {
        let p = plan(&loaded.prefs);
        let mut text = Vec::new();
        text.push(if p.would_refuse { format!("Would refuse: {}", p.reason) } else { p.summary.clone() });
        text.extend(p.changes.iter().map(|c| format!("  [{}] {} — {}", c.group, c.label, c.detail)));
        return Output::ok(p.to_json(), text.join("\n"));
    }
    let mut prefs = loaded.prefs;
    let mut result = match change(&mut prefs) {
        Ok(v) => v,
        Err(e) => return Output::fail(e),
    };
    match store::save(&prefs) {
        Ok(path) => {
            let summary = result["summary"].as_str().unwrap_or("Saved").to_string();
            result["ok"] = json!(true);
            result["path"] = json!(path.display().to_string());
            Output::ok(result, summary)
        }
        Err(e) => Output::fail(e),
    }
}

fn need<'a>(args: &'a Args, n: usize, what: &str) -> Result<&'a [String], String> {
    if args.words.len() < n {
        Err(format!("missing {what}"))
    } else {
        Ok(&args.words[..])
    }
}

fn config(args: &Args) -> Output {
    let sub = args.words.get(1).map(String::as_str).unwrap_or("show");
    match sub {
        "show" => match store::load() {
            Ok(l) => {
                let json = json!({"saved": l.path.is_some(), "settings": config_view(&l.prefs)});
                let mut text = String::new();
                for s in settings::all() {
                    text += &format!("{} = {}\n", s.key, settings::current(&s, &l.prefs));
                }
                Output::ok(json, text.trim_end())
            }
            Err(e) => Output::fail(e),
        },
        "set" | "reset" | "add" | "remove" => {
            let words = match need(args, if sub == "reset" { 3 } else { 4 }, "a key and value") {
                Ok(w) => w,
                Err(e) => return Output::fail(e),
            };
            let Some(setting) = settings::find(&words[2]) else {
                return Output::fail(format!("unknown setting \"{}\"", words[2]));
            };
            if let Some(why) = setting.readonly {
                return Output::fail(format!("{} can't be changed here. {why}", setting.key));
            }
            let is_list = matches!(setting.kind, Kind::Apps);
            if matches!(sub, "add" | "remove") != is_list && sub != "reset" {
                return Output::fail(if is_list {
                    format!("{} is a list: use config add or config remove", setting.key)
                } else {
                    format!("{} is not a list: use config set", setting.key)
                });
            }
            let key = setting.key;
            let next = |prefs: &Prefs| -> Result<Value, String> {
                let now = settings::current(&setting, prefs);
                match sub {
                    "reset" => Ok((setting.default)()),
                    "set" => settings::parse(&setting, &words[3]),
                    _ => {
                        let app = words[3].trim().to_string();
                        if app.is_empty() || app.chars().count() > settings::MAX_EXCLUDED_APP_LEN || app.chars().any(char::is_control) {
                            return Err("An app name is 1 to 128 characters, with no control characters.".into());
                        }
                        let mut list: Vec<String> = settings::clean_apps(&now);
                        let at = list.iter().position(|a| a.eq_ignore_ascii_case(&app));
                        if sub == "add" {
                            if at.is_some() {
                                return Err(format!("{app} is already in the list."));
                            }
                            if list.len() >= settings::MAX_EXCLUDED_APPS {
                                return Err(format!("The list holds at most {} apps.", settings::MAX_EXCLUDED_APPS));
                            }
                            list.push(app);
                        } else {
                            list.remove(at.ok_or_else(|| format!("{app} is not in the list."))?);
                        }
                        Ok(json!(list))
                    }
                }
            };
            let plan = |prefs: &Prefs| match next(prefs) {
                Ok(v) => plan::Plan::new(
                    format!("Change {key}"),
                    vec![plan::Change {
                        group: "Settings",
                        label: key.to_string(),
                        detail: format!("{} → {v}", settings::current(&setting, prefs)),
                        reversible: true,
                    }],
                ),
                Err(e) => plan::Plan::refuse(e),
            };
            mutate(args, plan, |prefs| {
                let value = next(prefs)?;
                let previous = settings::current(&setting, prefs);
                prefs.insert(key.into(), value.clone());
                // The app keeps no more recents than the limit allows.
                if key == "recentMax" {
                    if let (Some(Value::Array(r)), Some(n)) = (prefs.get_mut("recents"), value.as_u64()) {
                        r.truncate(n as usize);
                    }
                }
                Ok(json!({"summary": format!("{key} is now {value}"), "key": key, "value": value, "previous": previous}))
            })
        }
        other => Output::fail(format!("unknown config command \"{other}\"")),
    }
}

fn macro_cmd(args: &Args) -> Output {
    let sub = args.words.get(1).map(String::as_str).unwrap_or("list");
    let flag = |n: &str| args.flags.get(n).cloned();
    let op: Result<Op, String> = match sub {
        "list" | "deleted" => {
            return match store::load() {
                Ok(l) => {
                    let views = if sub == "list" { macros::macro_views(&l.prefs) } else { macros::deleted_views(&l.prefs) };
                    let text = views
                        .iter()
                        .map(|m| format!("{}  {}  {}", m["id"].as_str().unwrap_or(""), m["trigger"].as_str().unwrap_or(""), m["expansion"].as_str().unwrap_or("")))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let key = if sub == "list" { "macros" } else { "deleted" };
                    Output::ok(json!({ key: views }), text)
                }
                Err(e) => Output::fail(e),
            };
        }
        "add" => need(args, 4, "a trigger and an expansion").map(|w| Op::Add {
            trigger: w[2].clone(),
            expansion: w[3].clone(),
            hotkey: flag("hotkey"),
        }),
        "set" => need(args, 3, "a macro").and_then(|w| {
            let enabled = match flag("enabled").as_deref() {
                None => None,
                Some("true") => Some(true),
                Some("false") => Some(false),
                Some(v) => return Err(format!("--enabled is true or false, not \"{v}\"")),
            };
            let (trigger, expansion, hotkey) = (flag("trigger"), flag("expansion"), flag("hotkey"));
            if enabled.is_none() && trigger.is_none() && expansion.is_none() && hotkey.is_none() {
                return Err("nothing to change: give --enabled, --trigger, --expansion or --hotkey".into());
            }
            Ok(Op::Set { id: w[2].clone(), enabled, trigger, expansion, hotkey })
        }),
        "remove" => need(args, 3, "a macro").map(|w| Op::Remove { id: w[2].clone() }),
        "restore" => need(args, 3, "a deleted macro").map(|w| Op::Restore { id: w[2].clone() }),
        "purge" => {
            if args.flags.contains_key("all") {
                Ok(Op::Purge { id: None })
            } else {
                need(args, 3, "a deleted macro, or --all").map(|w| Op::Purge { id: Some(w[2].clone()) })
            }
        }
        other => Err(format!("unknown macro command \"{other}\"")),
    };
    match op {
        Err(e) => Output::fail(e),
        Ok(op) => mutate(args, |p| macros::plan(p, &op), |p| macros::apply(p, &op, now_ms())),
    }
}

fn dispatch(args: &Args) -> Output {
    match args.words.first().map(String::as_str) {
        Some("status") => match store::load() {
            Ok(l) => status(&l),
            Err(e) => Output::fail(e),
        },
        Some("config") => config(args),
        Some("macro") => macro_cmd(args),
        _ => Output::fail("unknown command"),
    }
}

/// Runs a command line (without the program name) and returns its output and whether it was
/// asked for as JSON. None when it isn't one of ours.
fn execute(args: &[String]) -> Option<(Output, bool)> {
    if !args.first().is_some_and(|a| COMMANDS.contains(&a.as_str())) {
        return None;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Some((Output::ok(json!({"usage": USAGE}), USAGE), false));
    }
    let wants_json = args.iter().any(|a| a == "--json");
    Some(match parse_args(args) {
        Ok(parsed) => (dispatch(&parsed), parsed.json),
        Err(e) => (Output::fail(e), wants_json),
    })
}

/// Runs a command line (without the program name). None when it isn't one of ours, so the app
/// starts as it always did; otherwise the exit code.
pub fn run(args: &[String]) -> Option<i32> {
    let (out, json) = execute(args)?;
    if json {
        println!("{}", out.json);
    } else if out.ok {
        println!("{}", out.text);
    } else {
        eprintln!("emobie: {}", out.text);
    }
    Some(if out.ok { 0 } else { 1 })
}

#[cfg(test)]
mod tests;
