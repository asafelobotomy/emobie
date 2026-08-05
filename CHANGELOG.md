# Changelog

All notable changes to Emobie are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/asafelobotomy/emobie/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/asafelobotomy/emobie/releases/tag/v0.5.0
