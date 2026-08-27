# Changelog

All notable changes to emobie are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Settings → Text expansion: optional **Keep Space after expansion** (`.hi` + Space → `hiya `)
- RPM packages ship the same `emobie-inputd` assets as `.deb`
- Flatpak: `xdg-data/emobie:ro` + host Grant path; KWin talk-name for Plasma pin

### Changed

- Text expansion controls live under Settings → Text expansion (not per-macro)
- Emoji shortcode macros stay collapsed unless searching or expanded
- Space-terminated expansion is the recommended default (e.g. `.hi` then Space)
- emobie-inputd starts with the app; enabling Expand as you type starts it if needed and turns listening on immediately
- Enabling Expand (or first-run setup) prompts once for keyboard access, restarts the helper, and skips logout when session ACLs apply
- Packaged setup uses `pkexec /usr/share/emobie/setup-input-access.sh` (matches Polkit policy)
- emobie-inputd listens on all keyboard devices and refreshes `can_listen` on each status query

### Fixed

- Pin (always-on-top) on Plasma Wayland via KWin `keepAbove` (GTK keep-above is a no-op there)
- Pin is re-applied after show/focus so it survives hide-to-tray
- Trailing spaces in expansions are typed as Space key events (more reliable than text inject)
- setfacl failures are reported instead of claiming ACLs always applied
- Flatpak Grant looks for the host-staged setup script via `flatpak-spawn --host`
- Flathub source manifest tag bumped to v0.6.5

## [0.6.5] - 2026-08-13

### Added

- Startup check against GitHub Releases for newer versions (toggle in Settings; dismissible)
- Text macros: Macros nav category, in-pane Add (+), per-macro hotkeys, emoji shortcodes + common emoticons (`:)`, `;')`, …), Espanso-ish YAML import/export
- Optional auto-paste on copy and as-you-type expansion via host helper `emobie-inputd` ([`docs/MACROS.md`](docs/MACROS.md))
- Secure `emobie-inputd` auto-start: systemd `--user` unit, owner-only socket + peer UID checks, install/setup scripts (helper starts with the app; Expand enables listening)
- First-launch setup dialog to start the input helper and optionally grant keyboard access
- Flatpak socket access to `$XDG_RUNTIME_DIR/emobie` for the host input helper (no `--device=input`)
- Flathub-bound offline source Flatpak manifest (`cargo-sources` / `node-sources`) alongside the GitHub Releases `.deb` wrap
- App screenshots and Flathub readiness checklist ([`docs/FLATHUB.md`](docs/FLATHUB.md)); submission remains deferred

### Changed

- Brand spelling normalized to lowercase `emobie` (product name, Flatpak app id `io.github.asafelobotomy.emobie`, docs, and packaging paths)

### Fixed

- Linux Flatpak tray registration on Cinnamon/Mint: disable StatusNotifierItem well-known name ownership in the sandbox and assume SNI via xapp-sn-watcher
- Clearer tray diagnostics in Settings when the tray fails to start

## [0.6.4] - 2026-08-07

### Added

- GitHub Releases now publish `.rpm` and AppImage alongside `.deb` and Flatpak

## [0.6.3] - 2026-08-07

### Added

- Allow multiple instances setting (hotkey disabled while enabled; restart to re-enforce single instance)
- Quit from Settings; close exits when the system tray is unavailable
- Reset usage stats in Settings
- CI per-file LOC gate (max 400 lines)

### Changed

- Start minimized is applied in Rust before the window paints (no flash)
- Sort label “Date added” renamed to “First used”
- Pin and Settings stay available while search is open
- Hotkeys require Ctrl, Alt, or Meta for letter/digit keys
- Preference normalization for theme, emoji size, skin tone, and hotkey
- CSS split into focused modules; Rust tray/prefs extracted under the 400 LOC rule

### Fixed

- Autostart desktop entry uses Name=emobie under Flatpak
- Autostart and preference save errors surface in Settings
- Resize handles no longer sit above the Settings dialog
- StartupWMClass set for better window manager grouping

## [0.6.2] - 2026-08-07

### Fixed

- Launch on startup under Flatpak now uses `flatpak run` instead of a sandbox-only path
- Start minimized is skipped when the system tray is unavailable, so the window cannot vanish
- Relaunching focuses the existing instance instead of stacking invisible copies

## [0.6.1] - 2026-08-05

### Added

- Launch on startup and start minimized to system tray settings
- Sort by preference (default, name, type, date added, number of uses)
- Resize edge cursors for the frameless window

### Fixed

- Pin button no longer rotates its frame when pinned

## [0.6.0] - 2026-08-05

### Fixed

- Frameless window can be moved by dragging the toolbar (brand / empty chrome)

## [0.5.5] - 2026-08-05

### Added

- Setting to show the OS title bar; hidden by default with toolbar drag

### Changed

- Branding wordmark is now `emobie`
- Toolbar search is always an icon / slim field next to the title
- Pin and Settings stay available in compact size, and hide while search is open

## [0.5.4] - 2026-08-05

### Added

- Doesbie branding icons across window, tray, favicon, and toolbar

### Changed

- Hotkeys accept any shortcut except bare letter/number keys (those still need a modifier)
- Linux tray uses StatusNotifierItem so left-click shows emobie; right-click opens the menu

## [0.5.3] - 2026-08-05

### Fixed

- Category strip scrollbar no longer overlays and obscures category icons

## [0.5.2] - 2026-08-05

### Fixed

- Flatpak crash on startup from missing Ayatana AppIndicator library
- System tray init no longer panics the whole app if AppIndicator is unavailable

## [0.5.1] - 2026-08-05

### Changed

- Flatpak runtime migrated from EOL GNOME 48 to GNOME Platform 50

## [0.5.0] - 2026-08-05

### Added

- Full Unicode emoji catalog with category navigation, search, and skin tones
- Favorites section with right-click favorite / unfavorite
- Recent emoji history with configurable size
- Always-on-top pin from the toolbar and system tray
- Global hotkey toggle (default `Ctrl+Shift+Space`), rebindable in Settings
- System tray with Show / Hide / Pin / Quit; window close hides to tray
- Adaptive layouts for horizontal, square, and vertical window shapes
- Compact chrome mode that keeps at least two emoji rows visible
- Light / dark / system themes and emoji size preferences
- Flatpak packaging (`io.github.asafelobotomy.emobie`)
- GitHub Actions release workflow that builds `.deb` + `.flatpak` artifacts

### Fixed

- Settings accessibility (Escape, backdrop click, focus)
- Clipboard and hotkey registration error feedback
- Preference load normalization for recents and favorites
- Horizontal mouse-wheel scrolling in wide layouts
- Tray icon temp path for Flatpak-friendly sandboxing

[Unreleased]: https://github.com/asafelobotomy/emobie/compare/v0.6.5...HEAD
[0.6.5]: https://github.com/asafelobotomy/emobie/compare/v0.6.4...v0.6.5
[0.6.4]: https://github.com/asafelobotomy/emobie/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/asafelobotomy/emobie/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/asafelobotomy/emobie/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/asafelobotomy/emobie/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/asafelobotomy/emobie/compare/v0.5.5...v0.6.0
[0.5.5]: https://github.com/asafelobotomy/emobie/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/asafelobotomy/emobie/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/asafelobotomy/emobie/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/asafelobotomy/emobie/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/asafelobotomy/emobie/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/asafelobotomy/emobie/releases/tag/v0.5.0
