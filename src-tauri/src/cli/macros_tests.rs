use super::*;

fn add(prefs: &mut Prefs, trigger: &str, expansion: &str) -> String {
    let r = apply(prefs, &Op::Add { trigger: trigger.into(), expansion: expansion.into(), hotkey: None }, 1).unwrap();
    r["macro"]["id"].as_str().unwrap().to_string()
}

#[test]
fn add_checks_what_the_app_checks() {
    let mut p = Prefs::new();
    add(&mut p, ".hi", "Hello");
    let dup = Op::Add { trigger: " .hi ".into(), expansion: "x".into(), hotkey: None };
    assert!(apply(&mut p, &dup, 1).unwrap_err().contains("already uses"));
    for (t, e) in [("", "x"), (".a\tb", "x"), (".a", ""), (".a", "x\0y")] {
        let op = Op::Add { trigger: t.into(), expansion: e.into(), hotkey: None };
        assert!(apply(&mut p, &op, 1).is_err(), "{t:?} {e:?}");
    }
    assert_eq!(macros(&p).len(), 1);
}

#[test]
fn ids_look_like_uuids() {
    let id = new_id();
    assert_eq!(id.len(), 36);
    assert_eq!(id.as_bytes()[14], b'4');
    assert_ne!(id, new_id());
}

#[test]
fn hotkeys_cannot_collide_with_the_summon_shortcut_or_each_other() {
    let mut p = Prefs::new();
    let op = |h: &str, t: &str| Op::Add { trigger: t.into(), expansion: "x".into(), hotkey: Some(h.into()) };
    assert!(apply(&mut p, &op("Control+Shift+Space", ".a"), 1).unwrap_err().contains("summon"));
    apply(&mut p, &op("Control+Alt+S", ".a"), 1).unwrap();
    assert!(apply(&mut p, &op("Control+Alt+S", ".b"), 1).unwrap_err().contains(".a"));
    assert!(apply(&mut p, &op("Shift+S", ".b"), 1).is_err());
}

#[test]
fn set_turns_off_and_on_and_edits() {
    let mut p = Prefs::new();
    let id = add(&mut p, ".hi", "Hello");
    let off = Op::Set { id: id.clone(), enabled: Some(false), trigger: None, expansion: None, hotkey: None };
    assert_eq!(apply(&mut p, &off, 1).unwrap()["summary"], "Turned off .hi");
    assert_eq!(macro_views(&p)[0]["enabled"], false);
    let edit = Op::Set { id: ".hi".into(), enabled: None, trigger: Some(".hey".into()), expansion: Some("Hey".into()), hotkey: Some("Control+Alt+H".into()) };
    apply(&mut p, &edit, 1).unwrap();
    let v = &macro_views(&p)[0];
    assert_eq!((v["trigger"].as_str(), v["expansion"].as_str(), v["hotkey"].as_str()), (Some(".hey"), Some("Hey"), Some("Control+Alt+H")));
    assert_eq!(v["enabled"], false, "an edit keeps the macro off");
    let clear = Op::Set { id, enabled: None, trigger: None, expansion: None, hotkey: Some("".into()) };
    apply(&mut p, &clear, 1).unwrap();
    assert!(macro_views(&p)[0]["hotkey"].is_null());
}

#[test]
fn remove_restore_and_purge_round_trip() {
    let mut p = Prefs::new();
    let a = add(&mut p, ".a", "A");
    let b = add(&mut p, ".b", "B");
    apply(&mut p, &Op::Remove { id: a.clone() }, 10).unwrap();
    apply(&mut p, &Op::Remove { id: b.clone() }, 20).unwrap();
    assert!(macros(&p).is_empty());
    let bin = deleted_views(&p);
    assert_eq!(bin[0]["trigger"], ".b", "newest first");
    assert_eq!(bin[0]["deleted_at"], 20);

    apply(&mut p, &Op::Restore { id: a.clone() }, 30).unwrap();
    assert_eq!(macro_views(&p)[0]["trigger"], ".a");
    assert!(macro_views(&p)[0].get("deleted_at").is_none());
    assert_eq!(deleted_views(&p).len(), 1);

    // The trigger was taken meanwhile: restoring is refused and nothing moves.
    let again = add(&mut p, ".b", "other");
    assert!(apply(&mut p, &Op::Restore { id: b.clone() }, 40).unwrap_err().contains("already uses"));
    assert_eq!(deleted_views(&p).len(), 1);
    apply(&mut p, &Op::Remove { id: again }, 50).unwrap();

    apply(&mut p, &Op::Purge { id: Some(b) }, 60).unwrap();
    assert_eq!(deleted_views(&p).len(), 1);
    let r = apply(&mut p, &Op::Purge { id: None }, 70).unwrap();
    assert_eq!(r["removed"], 1);
    assert!(deleted_views(&p).is_empty());
}

#[test]
fn the_bin_keeps_the_newest_fifty() {
    let mut p = Prefs::new();
    for n in 0..(DELETED_MAX + 5) {
        let id = add(&mut p, &format!(".m{n}"), "x");
        apply(&mut p, &Op::Remove { id }, n as u64 + 1).unwrap();
    }
    let bin = deleted_views(&p);
    assert_eq!(bin.len(), DELETED_MAX);
    assert_eq!(bin[0]["trigger"], format!(".m{}", DELETED_MAX + 4));
    assert!(bin.iter().all(|m| m["trigger"] != ".m0"));
}

#[test]
fn plans_say_what_would_happen_and_what_would_be_refused() {
    let mut p = Prefs::new();
    let id = add(&mut p, ".a", "A");
    let plan_remove = plan(&p, &Op::Remove { id: id.clone() });
    assert!(!plan_remove.would_refuse && plan_remove.changes[0].reversible);
    apply(&mut p, &Op::Remove { id: id.clone() }, 1).unwrap();
    let purge = plan(&p, &Op::Purge { id: None });
    assert!(!purge.changes[0].reversible);
    assert!(plan(&p, &Op::Remove { id: "nope".into() }).would_refuse);
    assert!(plan(&Prefs::new(), &Op::Purge { id: None }).would_refuse);
    assert!(plan(&p, &Op::Add { trigger: "".into(), expansion: "x".into(), hotkey: None }).would_refuse);
}

#[test]
fn unusable_entries_are_ignored_like_the_app_does() {
    let mut p = Prefs::new();
    p.insert("macros".into(), json!([
        {"id": "1", "trigger": ".ok", "expansion": "x"},
        {"id": "2", "trigger": ".ok", "expansion": "dup trigger"},
        {"id": "", "trigger": ".noid", "expansion": "x"},
        {"id": "3", "trigger": ".bad\u{1}", "expansion": "x"},
        "junk"
    ]));
    assert_eq!(macros(&p).len(), 1);
}
