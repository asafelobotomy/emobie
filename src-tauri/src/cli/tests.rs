use super::*;
use crate::cli::testenv::with_home;

fn run_json(line: &str) -> (bool, Value) {
    let args: Vec<String> = line.split_whitespace().map(String::from).collect();
    let (out, json) = execute(&args).expect("a command of ours");
    assert!(json || !line.contains("--json"));
    (out.ok, out.json)
}

fn run_args(args: &[&str]) -> (bool, Value) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (out, _) = execute(&args).unwrap();
    (out.ok, out.json)
}

#[test]
fn other_command_lines_start_the_app() {
    assert!(execute(&[]).is_none());
    assert!(execute(&["--minimized".to_string()]).is_none());
}

#[test]
fn status_before_anything_is_saved_shows_defaults() {
    with_home("status0", |_| {
        let (ok, v) = run_json("status --json");
        assert!(ok);
        assert_eq!(v["app"]["saved"], false);
        assert_eq!(v["config"]["theme"]["value"], "system");
        assert_eq!(v["config"]["expandAsYouType"]["readonly"], true);
        assert_eq!(v["macros"], json!([]));
        assert_eq!(v["deleted"], json!([]));
    });
}

#[test]
fn config_set_saves_validates_and_resets() {
    with_home("config", |_| {
        let (ok, v) = run_json("config set theme dark --json");
        assert!(ok, "{v}");
        assert_eq!(v["previous"], "system");
        assert_eq!(run_json("status --json").1["config"]["theme"]["value"], "dark");
        assert_eq!(run_json("status --json").1["config"]["theme"]["saved"], true);

        let (ok, v) = run_json("config set theme blue --json");
        assert!(!ok);
        assert!(v["error"].as_str().unwrap().contains("system, light, dark"));
        let (ok, v) = run_json("config set autoPasteOnCopy on --json");
        assert!(!ok && v["error"].as_str().unwrap().contains("in emobie itself"));
        assert!(!run_json("config set nope 1 --json").0);
        assert!(!run_json("config add theme x --json").0);

        // The plan changes nothing.
        let (ok, plan) = run_json("config set theme light --dry-run --json");
        assert!(ok);
        assert_eq!(plan["would_refuse"], false);
        assert_eq!(plan["changes"][0]["detail"], "\"dark\" → \"light\"");
        assert_eq!(run_json("status --json").1["config"]["theme"]["value"], "dark");
        assert_eq!(run_json("config set theme blue --dry-run --json").1["would_refuse"], true);

        assert!(run_json("config reset theme --json").0);
        assert_eq!(run_json("status --json").1["config"]["theme"]["value"], "system");
    });
}

#[test]
fn a_smaller_recent_limit_trims_the_recents() {
    with_home("recents", |_| {
        let mut p = Prefs::new();
        p.insert("recentMax".into(), json!(32));
        p.insert("recents".into(), json!((0..20).map(|n| format!("e{n}")).collect::<Vec<_>>()));
        store::save(&p).unwrap();
        assert!(run_json("config set recentMax 8 --json").0);
        assert_eq!(store::load().unwrap().prefs["recents"].as_array().unwrap().len(), 8);
    });
}

#[test]
fn the_excluded_apps_list_is_edited_with_add_and_remove() {
    with_home("apps", |_| {
        let (ok, _) = run_json("config add expandExcludedApps mything --json");
        assert!(ok);
        let (_, v) = run_json("status --json");
        let list = v["config"]["expandExcludedApps"]["value"].as_array().unwrap().clone();
        assert!(list.contains(&json!("mything")) && list.contains(&json!("keepassxc")));
        assert!(!run_json("config add expandExcludedApps MYTHING --json").0, "no case-insensitive duplicates");
        assert!(run_json("config remove expandExcludedApps mything --json").0);
        assert!(!run_json("config remove expandExcludedApps mything --json").0);
        assert!(!run_json("config set expandExcludedApps a --json").0);
    });
}

#[test]
fn macros_go_through_add_edit_delete_restore_and_purge() {
    with_home("macros", |_| {
        let (ok, v) = run_args(&["macro", "add", "--json", ".sig", "Best regards", "--hotkey", "Control+Alt+S"]);
        assert!(ok, "{v}");
        let id = v["macro"]["id"].as_str().unwrap().to_string();
        assert!(!run_args(&["macro", "add", "--json", ".sig", "again"]).0, "same trigger");

        let (ok, _) = run_args(&["macro", "set", "--json", "--enabled", "false", &id]);
        assert!(ok);
        assert_eq!(run_json("macro list --json").1["macros"][0]["enabled"], false);

        let (_, plan) = run_args(&["macro", "remove", "--dry-run", "--json", &id]);
        assert_eq!(plan["changes"][0]["reversible"], true);
        assert_eq!(run_json("macro list --json").1["macros"].as_array().unwrap().len(), 1, "a plan changes nothing");

        assert!(run_args(&["macro", "remove", "--json", &id]).0);
        let (_, st) = run_json("status --json");
        assert_eq!(st["macros"], json!([]));
        assert_eq!(st["deleted"][0]["trigger"], ".sig");
        assert!(st["deleted"][0]["deleted_at"].as_u64().unwrap() > 0);

        assert!(run_args(&["macro", "restore", "--json", &id]).0);
        assert_eq!(run_json("macro deleted --json").1["deleted"], json!([]));
        assert_eq!(run_json("macro list --json").1["macros"][0]["expansion"], "Best regards");

        assert!(run_args(&["macro", "remove", "--json", ".sig"]).0, "a trigger names a macro too");
        let (_, plan) = run_args(&["macro", "purge", "--all", "--dry-run", "--json"]);
        assert_eq!(plan["changes"][0]["reversible"], false);
        assert!(run_args(&["macro", "purge", "--all", "--json"]).0);
        assert_eq!(run_json("macro deleted --json").1["deleted"], json!([]));
        assert!(run_args(&["macro", "purge", "--all", "--dry-run", "--json"]).1["would_refuse"] == true);
    });
}

#[test]
fn macros_and_the_durable_mirror_stay_in_step() {
    with_home("mirror", |home| {
        assert!(run_args(&["macro", "add", "--json", ".a", "A"]).0);
        let mirror = std::fs::read_to_string(home.join(".local/share/emobie/preferences.json")).unwrap();
        assert!(mirror.contains("\".a\""));
    });
}

#[test]
fn bad_command_lines_are_json_errors_too() {
    with_home("errors", |_| {
        for args in [
            vec!["macro", "frobnicate", "--json"],
            vec!["macro", "set", "--json", "x"],
            vec!["macro", "set", "--json", "--enabled", "maybe", "x"],
            vec!["macro", "add", "--json", ".only-trigger"],
            vec!["macro", "add", "--json", "--bogus", ".a", "b"],
            vec!["config", "set", "--json", "theme"],
        ] {
            let (ok, v) = run_args(&args);
            assert!(!ok, "{args:?}");
            assert_eq!(v["ok"], false);
            assert!(v["error"].as_str().is_some_and(|e| !e.is_empty()), "{args:?}");
        }
    });
}

#[test]
fn text_starting_with_dashes_can_follow_a_double_dash() {
    with_home("dashes", |_| {
        assert!(run_args(&["macro", "add", "--json", "--", "--x", "dashes"]).0);
        assert_eq!(run_json("macro list --json").1["macros"][0]["trigger"], "--x");
    });
}

// ---- The uQ Settings page's test fixtures are the real output of these commands ----

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/uq/fixtures")
}

/// The output with what changes from run to run (temp folder, version) made fixed.
fn steady(mut v: Value, home: &std::path::Path) -> Value {
    fn walk(v: &mut Value, home: &str) {
        match v {
            Value::String(s) => *s = s.replace(home, "/home/user"),
            Value::Array(a) => a.iter_mut().for_each(|x| walk(x, home)),
            Value::Object(o) => o.values_mut().for_each(|x| walk(x, home)),
            _ => {}
        }
    }
    walk(&mut v, &home.display().to_string());
    if let Some(app) = v.get_mut("app") {
        app["version"] = json!("0.0.0");
    }
    v
}

fn check_fixture(name: &str, actual: &Value) {
    let path = fixture_dir().join(name);
    let text = format!("{}\n", serde_json::to_string_pretty(actual).unwrap());
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is missing: run with UPDATE_FIXTURES=1", path.display()));
    assert_eq!(want, text, "{name} is out of date: run the tests with UPDATE_FIXTURES=1");
}

#[test]
fn the_page_fixtures_are_what_the_commands_print() {
    with_home("fixtures", |home| {
        check_fixture("status-empty.json", &steady(run_json("status --json").1, home));

        let mut p = Prefs::new();
        p.insert("theme".into(), json!("dark"));
        p.insert("hotkey".into(), json!("Control+Alt+E"));
        p.insert("recentMax".into(), json!(40));
        p.insert("expandAsYouType".into(), json!(true));
        p.insert("expandExcludedApps".into(), json!(["keepassxc", "my-terminal"]));
        p.insert("macros".into(), json!([
            {"id": "m1", "trigger": ".sig", "expansion": "Best regards,\nSam", "hotkey": "Control+Alt+S", "enabled": true},
            {"id": "m2", "trigger": ".addr", "expansion": "1 Example Street", "hotkey": null, "enabled": false}
        ]));
        p.insert("deletedMacros".into(), json!([
            {"id": "d1", "trigger": ".thanks", "expansion": "Thank you!", "hotkey": null, "enabled": true, "deletedAt": 1_700_000_100_000u64},
            {"id": "d2", "trigger": ".old", "expansion": "an old one", "hotkey": "Control+Alt+O", "enabled": true, "deletedAt": 1_700_000_000_000u64}
        ]));
        store::save(&p).unwrap();
        check_fixture("status.json", &steady(run_json("status --json").1, home));

        let plan = |args: &[&str]| run_args(args).1;
        check_fixture("plan-remove.json", &plan(&["macro", "remove", "--dry-run", "--json", "m1"]));
        check_fixture("plan-purge.json", &plan(&["macro", "purge", "--dry-run", "--json", "d1"]));
        check_fixture("plan-purge-all.json", &plan(&["macro", "purge", "--all", "--dry-run", "--json"]));
        check_fixture("plan-refused.json", &plan(&["macro", "remove", "--dry-run", "--json", "nope"]));
        check_fixture("error-theme.json", &run_args(&["config", "set", "--json", "theme", "blue"]).1);
        check_fixture("error-trigger.json", &run_args(&["macro", "add", "--json", ".sig", "again"]).1);
        check_fixture("error-restore.json", &run_args(&["macro", "restore", "--json", "zzz"]).1);
    });
}
