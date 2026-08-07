# Changelog

All notable changes to Emobie are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- Linux tray uses StatusNotifierItem so left-click shows Emobie; right-click opens the menu

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
- Flatpak packaging (`io.github.asafelobotomy.Emobie`)
- GitHub Actions release workflow that builds `.deb` + `.flatpak` artifacts

### Fixed

- Settings accessibility (Escape, backdrop click, focus)
- Clipboard and hotkey registration error feedback
- Preference load normalization for recents and favorites
- Horizontal mouse-wheel scrolling in wide layouts
- Tray icon temp path for Flatpak-friendly sandboxing

[Unreleased]: https://github.com/asafelobotomy/emobie/compare/v0.6.2...HEAD
[0.6.2]: https://github.com/asafelobotomy/emobie/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/asafelobotomy/emobie/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/asafelobotomy/emobie/compare/v0.5.5...v0.6.0
[0.5.5]: https://github.com/asafelobotomy/emobie/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/asafelobotomy/emobie/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/asafelobotomy/emobie/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/asafelobotomy/emobie/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/asafelobotomy/emobie/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/asafelobotomy/emobie/releases/tag/v0.5.0
