use super::*;

fn with_layout(layout: &str, variant: &str, group: u32) -> KeymapState {
    let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap =
        xkb::Keymap::new_from_names(&ctx, "evdev", "pc105", layout, variant, None, KEYMAP_COMPILE_NO_FLAGS)
            .expect("test keymap (needs xkeyboard-config)");
    let mut state = xkb::State::new(&keymap);
    state.update_mask(0, 0, 0, 0, 0, group);
    KeymapState {
        state: Mutex::new(XkbState(state)),
        fingerprint: Mutex::new(String::new()),
        seen_generation: AtomicU64::new(0),
        last_periodic_reload: AtomicU64::new(0),
        pressed: Mutex::new(HashSet::new()),
    }
}

fn plan(km: &KeymapState, c: char) -> Option<(u16, &'static [u16])> {
    km.plan_char(c).map(|p| (p.code, p.mods))
}

const SHIFT: &[u16] = &[KEY_LEFTSHIFT];
const ALTGR: &[u16] = &[KEY_RIGHTALT];
const NONE: &[u16] = &[];

#[test]
fn us_layout_plans() {
    let km = with_layout("us", "", 0);
    assert_eq!(plan(&km, 'a'), Some((30, NONE)));
    assert_eq!(plan(&km, 'A'), Some((30, SHIFT)));
    assert_eq!(plan(&km, '@'), Some((3, SHIFT)));
    // Main-row Shift+8, not the keypad asterisk.
    assert_eq!(plan(&km, '*'), Some((9, SHIFT)));
    assert_eq!(plan(&km, ' '), Some((57, NONE)));
}

#[test]
fn untypeable_chars_have_no_plan() {
    let km = with_layout("us", "", 0);
    assert_eq!(plan(&km, 'é'), None);
    assert_eq!(plan(&km, '😀'), None);
    assert_eq!(plan(&km, '\n'), None);
}

#[test]
fn gb_symbols_differ_from_us() {
    let km = with_layout("gb", "", 0);
    assert_eq!(plan(&km, '@'), Some((40, SHIFT)));
    assert_eq!(plan(&km, '"'), Some((3, SHIFT)));
    assert_eq!(plan(&km, '£'), Some((4, SHIFT)));
}

#[test]
fn de_uses_qwertz_and_altgr() {
    let km = with_layout("de", "", 0);
    assert_eq!(plan(&km, 'z'), Some((21, NONE)));
    assert_eq!(plan(&km, 'ß'), Some((12, NONE)));
    assert_eq!(plan(&km, '@'), Some((16, ALTGR)));
}

#[test]
fn active_group_is_honoured() {
    let km = with_layout("us,de", "", 1);
    assert_eq!(plan(&km, 'z'), Some((21, NONE)));
    let km = with_layout("us,de", "", 0);
    assert_eq!(plan(&km, 'z'), Some((44, NONE)));
}

#[test]
fn caps_lock_inverts_letter_case() {
    let km = with_layout("us", "", 0);
    km.update_key(58, true);
    km.update_key(58, false);
    assert_eq!(plan(&km, 'A'), Some((30, NONE)));
    assert_eq!(plan(&km, 'a'), Some((30, SHIFT)));
}

#[test]
fn keymap_state_maps_ascii_key() {
    let km = with_layout("us", "", 0);
    km.update_key(30, true);
    assert_eq!(km.key_utf8(30).as_deref(), Some("a"));
    km.update_key(30, false);
}

#[test]
fn reload_skipped_while_key_held() {
    let km = KeymapState::new();
    km.update_key(30, true);
    assert!(!km.reload_from_session_if_idle());
    km.update_key(30, false);
}

#[test]
fn autorepeat_does_not_wedge_reload() {
    let km = KeymapState::new();
    // Held key re-reported as pressed several times (autorepeat), then released.
    for _ in 0..5 {
        km.update_key(30, true);
    }
    assert!(km.any_key_held());
    km.update_key(30, false);
    assert!(!km.any_key_held(), "one release must clear a repeated press");
}

#[test]
fn ctrl_counts_as_shortcut_but_altgr_does_not() {
    let km = with_layout("de", "", 0);
    km.update_key(29, true); // Left Ctrl
    assert!(km.shortcut_modifier_held());
    km.update_key(29, false);
    km.update_key(KEY_RIGHTALT, true); // AltGr on de
    assert!(!km.shortcut_modifier_held());
    km.update_key(KEY_RIGHTALT, false);
}

#[test]
fn autorepeated_modifier_is_released_by_one_up() {
    let km = with_layout("us", "", 0);
    for _ in 0..10 {
        km.update_key(56, true); // Left Alt held: press + autorepeats
    }
    assert!(km.shortcut_modifier_held());
    km.update_key(56, false);
    assert!(!km.shortcut_modifier_held(), "Alt stuck after autorepeat");
    km.update_key(42, false); // spurious Up for a key never pressed
    assert_eq!(km.key_utf8(30).as_deref(), Some("a"));
}
