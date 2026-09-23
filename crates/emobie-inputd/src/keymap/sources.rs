//! Where the session keyboard layout comes from.
//!
//! Resolution order:
//! 1. `XKB_DEFAULT_*` (explicit override)
//! 2. GNOME `org.gnome.desktop.input-sources` (including the active source)
//! 3. Plasma `~/.config/kxkbrc` (active index from `org.kde.keyboard`)
//! 4. `/etc/X11/xorg.conf.d/00-keyboard.conf` (written by `localectl`)
//! 5. `/etc/default/keyboard`, `/etc/vconsole.conf`
//! 6. libxkbcommon defaults

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// gnome-shell packs input sources into keymaps of at most 4 layouts and
/// locks the group of the active one (see its `keyboard.js`).
const GNOME_MAX_LAYOUTS_PER_GROUP: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Rmlvo {
    pub rules: String,
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: String,
    /// Locked layout group that is active right now.
    pub group: u32,
}

impl Rmlvo {
    pub fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.rules, self.model, self.layout, self.variant, self.options, self.group
        )
    }
}

fn desktop_has(name: &str) -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|part| part.eq_ignore_ascii_case(name))
}

pub fn is_gnome() -> bool {
    desktop_has("GNOME")
}

pub fn is_plasma() -> bool {
    desktop_has("KDE") || desktop_has("Plasma")
}

pub fn resolve(plasma_group: u32) -> Rmlvo {
    from_env()
        .or_else(|| is_gnome().then(from_gnome).flatten())
        .or_else(|| from_kxkbrc(plasma_group))
        .or_else(|| from_xorg_conf("/etc/X11/xorg.conf.d/00-keyboard.conf"))
        .or_else(from_default_keyboard)
        .unwrap_or_default()
}

fn from_env() -> Option<Rmlvo> {
    let layout = std::env::var("XKB_DEFAULT_LAYOUT").ok()?;
    if layout.trim().is_empty() {
        return None;
    }
    Some(Rmlvo {
        rules: std::env::var("XKB_DEFAULT_RULES").unwrap_or_default(),
        model: std::env::var("XKB_DEFAULT_MODEL").unwrap_or_else(|_| "pc105".into()),
        layout,
        variant: std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default(),
        options: std::env::var("XKB_DEFAULT_OPTIONS").unwrap_or_default(),
        group: 0,
    })
}

fn gsettings_get(key: &str) -> Option<String> {
    let out = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.input-sources", key])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Parses gsettings' `a(ss)` text form, e.g. `[('xkb', 'us'), ('ibus', 'anthy')]`.
pub(super) fn parse_source_list(raw: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find('(') {
        let Some(end) = rest[start..].find(')') else {
            break;
        };
        let inner = &rest[start + 1..start + end];
        let parts: Vec<String> = inner
            .split(',')
            .map(|p| p.trim().trim_matches('\'').to_string())
            .collect();
        if let [kind, id] = parts.as_slice() {
            out.push((kind.clone(), id.clone()));
        }
        rest = &rest[start + end + 1..];
    }
    out
}

/// Parses gsettings' `as` text form, e.g. `['compose:ralt', 'caps:escape']`.
fn parse_string_list(raw: &str) -> Vec<String> {
    let inner = raw.trim().trim_start_matches("@as").trim();
    let inner = inner.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|p| p.trim().trim_matches('\'').to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Turns GNOME's source list plus the active source into RMLVO fields.
/// IBus engines have no xkb id here; they type through the `us` layout.
pub(super) fn gnome_rmlvo(
    sources: &[(String, String)],
    active: Option<&(String, String)>,
    options: &[String],
) -> Option<Rmlvo> {
    if sources.is_empty() {
        return None;
    }
    let active_idx = active
        .and_then(|a| sources.iter().position(|s| s == a))
        .unwrap_or(0);
    let chunk_start = active_idx - active_idx % GNOME_MAX_LAYOUTS_PER_GROUP;
    let chunk = &sources[chunk_start..(chunk_start + GNOME_MAX_LAYOUTS_PER_GROUP).min(sources.len())];
    let mut layouts = Vec::new();
    let mut variants = Vec::new();
    for (kind, id) in chunk {
        let (layout, variant) = if kind == "xkb" {
            id.split_once('+').unwrap_or((id.as_str(), ""))
        } else {
            ("us", "")
        };
        layouts.push(layout.to_string());
        variants.push(variant.to_string());
    }
    Some(Rmlvo {
        rules: "evdev".into(),
        model: "pc105".into(),
        layout: layouts.join(","),
        variant: variants.join(","),
        options: options.join(","),
        group: (active_idx - chunk_start) as u32,
    })
}

fn from_gnome() -> Option<Rmlvo> {
    let sources = parse_source_list(&gsettings_get("sources")?);
    // gnome-shell keeps the active source first in `mru-sources`.
    let mru = gsettings_get("mru-sources")
        .map(|raw| parse_source_list(&raw))
        .unwrap_or_default();
    let options = gsettings_get("xkb-options")
        .map(|raw| parse_string_list(&raw))
        .unwrap_or_default();
    gnome_rmlvo(&sources, mru.first(), &options)
}

fn from_kxkbrc(plasma_group: u32) -> Option<Rmlvo> {
    let home = std::env::var_os("HOME")?;
    let raw = fs::read_to_string(PathBuf::from(home).join(".config/kxkbrc")).ok()?;
    let mut rmlvo = Rmlvo {
        rules: "evdev".into(),
        model: "pc105".into(),
        ..Rmlvo::default()
    };
    for line in raw.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("LayoutList=") {
            rmlvo.layout = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("VariantList=") {
            rmlvo.variant = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Options=") {
            rmlvo.options = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Model=") {
            if !rest.trim().is_empty() {
                rmlvo.model = rest.trim().to_string();
            }
        }
    }
    if rmlvo.layout.is_empty() {
        return None;
    }
    let count = rmlvo.layout.split(',').count() as u32;
    rmlvo.group = if plasma_group < count { plasma_group } else { 0 };
    Some(rmlvo)
}

pub(super) fn parse_xorg_conf(raw: &str) -> Option<Rmlvo> {
    let mut rmlvo = Rmlvo {
        rules: "evdev".into(),
        model: "pc105".into(),
        ..Rmlvo::default()
    };
    for line in raw.lines().map(str::trim) {
        let Some(rest) = line.strip_prefix("Option") else {
            continue;
        };
        let quoted: Vec<&str> = rest.split('"').skip(1).step_by(2).collect();
        let [key, value] = quoted.as_slice() else {
            continue;
        };
        match *key {
            "XkbLayout" => rmlvo.layout = value.to_string(),
            "XkbVariant" => rmlvo.variant = value.to_string(),
            "XkbModel" => rmlvo.model = value.to_string(),
            "XkbOptions" => rmlvo.options = value.to_string(),
            _ => {}
        }
    }
    (!rmlvo.layout.is_empty()).then_some(rmlvo)
}

fn from_xorg_conf(path: &str) -> Option<Rmlvo> {
    parse_xorg_conf(&fs::read_to_string(path).ok()?)
}

fn from_default_keyboard() -> Option<Rmlvo> {
    ["/etc/default/keyboard", "/etc/vconsole.conf"]
        .into_iter()
        .find_map(|path| parse_keyboard_file(&fs::read_to_string(path).ok()?))
}

pub(super) fn parse_keyboard_file(raw: &str) -> Option<Rmlvo> {
    let mut rmlvo = Rmlvo {
        rules: "evdev".into(),
        model: "pc105".into(),
        ..Rmlvo::default()
    };
    let mut console_keymap = None;
    for line in raw.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "XKBLAYOUT" => rmlvo.layout = value.to_string(),
            // vconsole `KEYMAP=uk` is only a hint when XKBLAYOUT is absent.
            "KEYMAP" => console_keymap = Some(if value == "uk" { "gb" } else { value }.to_string()),
            "XKBVARIANT" => rmlvo.variant = value.to_string(),
            "XKBMODEL" if !value.is_empty() => rmlvo.model = value.to_string(),
            "XKBOPTIONS" => rmlvo.options = value.to_string(),
            _ => {}
        }
    }
    if rmlvo.layout.is_empty() {
        rmlvo.layout = console_keymap?;
    }
    (!rmlvo.layout.is_empty()).then_some(rmlvo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(kind: &str, id: &str) -> (String, String) {
        (kind.into(), id.into())
    }

    #[test]
    fn parses_gnome_sources() {
        assert_eq!(parse_source_list("@a(ss) []"), vec![]);
        assert_eq!(
            parse_source_list("[('xkb', 'us'), ('xkb', 'gb+extd'), ('ibus', 'anthy')]"),
            vec![src("xkb", "us"), src("xkb", "gb+extd"), src("ibus", "anthy")]
        );
    }

    #[test]
    fn gnome_active_source_selects_group() {
        let sources = [src("xkb", "us"), src("xkb", "de+nodeadkeys")];
        let r = gnome_rmlvo(&sources, Some(&src("xkb", "de+nodeadkeys")), &[]).unwrap();
        assert_eq!(r.layout, "us,de");
        assert_eq!(r.variant, ",nodeadkeys");
        assert_eq!(r.group, 1);
        let first = gnome_rmlvo(&sources, None, &["caps:escape".into()]).unwrap();
        assert_eq!(first.group, 0);
        assert_eq!(first.options, "caps:escape");
    }

    #[test]
    fn gnome_chunks_of_four() {
        let sources: Vec<_> = ["us", "gb", "de", "fr", "es", "it"]
            .iter()
            .map(|l| src("xkb", l))
            .collect();
        let r = gnome_rmlvo(&sources, Some(&src("xkb", "it")), &[]).unwrap();
        assert_eq!(r.layout, "es,it");
        assert_eq!(r.group, 1);
    }

    #[test]
    fn parses_localectl_xorg_conf() {
        let raw = "Section \"InputClass\"\n  Option \"XkbLayout\" \"gb\"\n  Option \"XkbModel\" \"pc105+inet\"\nEndSection";
        let r = parse_xorg_conf(raw).unwrap();
        assert_eq!(r.layout, "gb");
        assert_eq!(r.model, "pc105+inet");
    }

    #[test]
    fn parses_default_keyboard() {
        let r = parse_keyboard_file("XKBLAYOUT=\"fr\"\nXKBVARIANT=\"azerty\"\nbad line\n").unwrap();
        assert_eq!((r.layout.as_str(), r.variant.as_str()), ("fr", "azerty"));
        let console = parse_keyboard_file("KEYMAP=uk\n").unwrap();
        assert_eq!(console.layout, "gb");
    }
}
