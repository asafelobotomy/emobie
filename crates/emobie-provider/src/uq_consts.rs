//! uq-consts 1.0.0
//!
//! The values uQ, Bindle and uQ's native apps share (paths, install folders, D-Bus names, the
//! `uq_api`, motion, shape, type and the fallback palette). Generated from uQ's
//! `native/uq-consts/consts.toml` by gen.py: vendor this file unchanged and declare it with
//! `#[rustfmt::skip] mod uq_consts;`. No dependencies; builds on editions 2021 and 2024.
#![allow(dead_code)]

/// The version of this file; `check-uq-native.sh` compares it with the one a repo pins.
pub const VERSION: &str = "1.0.0";

/// Per-user paths. `state_home()` is `$XDG_STATE_HOME` when it is absolute, else
/// `~/.local/state`; `cache_home()` likewise with `$XDG_CACHE_HOME` and `~/.cache`.
pub mod paths {
    use std::ffi::OsString;
    use std::path::PathBuf;

    /// Relative to `state_home()`.
    pub const UQ_DIR: &str = "uq";
    /// Relative to `state_home()`.
    pub const THEME_DIR: &str = "uq/theme";
    /// Relative to `state_home()`.
    pub const TOKENS: &str = "uq/theme/tokens.json";
    /// Relative to `state_home()`.
    pub const CRASHES_DIR: &str = "uq/crashes";
    /// Relative to `cache_home()`.
    pub const QUICKSHELL_CRASHES_DIR: &str = "quickshell/crashes";

    fn base(get: &dyn Fn(&str) -> Option<OsString>, var: &str, home_rel: &str) -> Option<PathBuf> {
        match get(var).map(PathBuf::from).filter(|p| p.is_absolute()) {
            Some(p) => Some(p),
            None => get("HOME").filter(|h| !h.is_empty()).map(|h| PathBuf::from(h).join(home_rel)),
        }
    }

    /// `state_home()` from an environment lookup, for tests.
    pub fn state_home_from(get: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        base(get, "XDG_STATE_HOME", ".local/state")
    }

    /// `cache_home()` from an environment lookup, for tests.
    pub fn cache_home_from(get: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        base(get, "XDG_CACHE_HOME", ".cache")
    }

    pub fn state_home() -> Option<PathBuf> {
        state_home_from(&|k| std::env::var_os(k))
    }

    pub fn cache_home() -> Option<PathBuf> {
        cache_home_from(&|k| std::env::var_os(k))
    }

    pub fn uq_dir() -> Option<PathBuf> {
        state_home().map(|b| b.join(UQ_DIR))
    }
    pub fn theme_dir() -> Option<PathBuf> {
        state_home().map(|b| b.join(THEME_DIR))
    }
    pub fn tokens() -> Option<PathBuf> {
        state_home().map(|b| b.join(TOKENS))
    }
    pub fn crashes_dir() -> Option<PathBuf> {
        state_home().map(|b| b.join(CRASHES_DIR))
    }
    pub fn quickshell_crashes_dir() -> Option<PathBuf> {
        cache_home().map(|b| b.join(QUICKSHELL_CRASHES_DIR))
    }
}

pub mod install {
    pub const DATA_DIR: &str = "/usr/share/uq";
    pub const PAGES_DIR: &str = "/usr/share/uq/settings/pages";
    pub const WIDGETS_DIR: &str = "/usr/share/uq/shell/widgets";
    pub const LAUNCHER_DIR: &str = "/usr/share/uq/launcher";
    pub const ACTIONS_DIR: &str = "/usr/share/uq/launcher/actions";
    pub const INTEGRATIONS_DIR: &str = "/usr/share/uq/integrations";
    pub const DIAG_DIR: &str = "/usr/share/uq/diag";
    pub const THEMES_DIR: &str = "/usr/share/uq/themes";
    pub const RENDERED_THEMES_DIR: &str = "/usr/share/uq/themes/rendered";
}

pub mod dbus {
    pub const DESKTOP_NAME: &str = "org.uq.Desktop1";
    pub const DESKTOP_PATH: &str = "/org/uq/Desktop1";
    pub const DESKTOP_INTERFACE: &str = "org.uq.Desktop1";
    pub const LAUNCHER_PROVIDER_PATH: &str = "/org/uq/Launcher1";
    pub const LAUNCHER_PROVIDER_INTERFACE: &str = "org.uq.Launcher1.Provider";
}

pub mod api {
    pub const UQ_API: u32 = 3;
    pub const PAGE_DEFAULT_API: u32 = 1;
}

pub mod motion {
    pub const FAST: u32 = 100;
    pub const BASE: u32 = 140;
    pub const SLOW: u32 = 220;
    pub const POP_SCALE: f64 = 0.96;
    pub const EASING: &str = "OutCubic";
    pub const EASING_EXIT: &str = "InCubic";
}

pub mod shape {
    pub const FALLBACK_RADIUS: u32 = 10;
    pub const CONTROL_INSET: u32 = 2;
    pub const INNER_INSET: u32 = 4;
    pub const GAP: u32 = 8;
}

pub mod type_scale {
    pub const FALLBACK_SIZE: u32 = 11;
    pub const SMALL_OFFSET: u32 = 1;
    pub const SMALL_MIN: u32 = 8;
    pub const HEADING_SCALE: f64 = 1.5;
}

/// uQ's Graphite theme, the palette to fall back to where uQ isn't running.
pub mod palette {
    /// The colour tokens every uQ theme defines, in this order.
    pub const KEYS: [&str; 12] = ["bg", "surface", "surface_alt", "overlay", "text", "text_muted", "accent", "accent_text", "border", "success", "warning", "error"];
    pub const DARK: [(&str, &str); 12] = [("bg", "#1f1f22"), ("surface", "#2b2b2e"), ("surface_alt", "#353538"), ("overlay", "#464649"), ("text", "#e4e4e7"), ("text_muted", "#a9a9ac"), ("accent", "#8ab4f8"), ("accent_text", "#1f1f22"), ("border", "#5a5a5d"), ("success", "#81c995"), ("warning", "#fdd663"), ("error", "#f28b82")];
    pub const LIGHT: [(&str, &str); 12] = [("bg", "#f4f4f5"), ("surface", "#e7e7e8"), ("surface_alt", "#dcdcdd"), ("overlay", "#c8c8c9"), ("text", "#18181b"), ("text_muted", "#5a5a5c"), ("accent", "#2b5bb8"), ("accent_text", "#f4f4f5"), ("border", "#b2b2b4"), ("success", "#1e7a3a"), ("warning", "#8a6100"), ("error", "#b3261e")];
}
