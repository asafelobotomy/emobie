# Changelog

All notable changes to emobie are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.18] - 2026-09-03

### Fixed

- AppImage: launched to a blank/aborting window on hosts with a newer Mesa than the
  CI build image — the bundled `libwayland-client`/`-cursor`/`-egl`/`-server` shadowed
  the host's via `LD_LIBRARY_PATH` and lacked symbols (`wl_fixes_interface`) the
  host's `libEGL_mesa` requires. Strip the bundled copies from the AppImage so it
  always uses the host's own, matching its Mesa driver
- Expand: rich-text/multi-line/emoji expansions (clipboard path) silently pasted
  empty on Wayland far more often than the trigger-detection logs suggested. Root
  cause was two-fold — `arboard` was built without the `wayland-data-control`
  feature, so on every Wayland session it silently fell back through XWayland's X11
  clipboard bridge instead of using native Wayland clipboard; and even once native,
  self-read-back "ready" checks don't prove the compositor has propagated the new
  selection to the focused client before Ctrl+V fires. Enabled the feature and added
  a settle delay on that path (measured ~17% silent-empty-paste rate before the fix,
  0/20+ after in repeated E2E runs)
- `verify-expand-setup.sh`: the keyboard-readability check could validate the app's
  own synthetic uinput device (`emobie-inject`, which also reports
  `ID_INPUT_KEYBOARD=1`) instead of real hardware, since `/dev/input/event*` globs
  lexicographically and the loop stopped at the first readable match — masking a
  genuinely broken real-keyboard permission with a false "all checks passed"

### Changed

- deb/rpm packages now `Recommends: wl-clipboard, acl`; Arch `PKGBUILD` lists them
  as `optdepends` — `wl-clipboard` gives Expand an externally-verified Wayland paste
  path (stronger than the in-process arboard check) when installed
- `verify-expand-setup.sh` warns when `wl-clipboard` is missing on a Wayland session

## [0.6.17] - 2026-09-03

### Fixed

- Release AppImage verify: host-bundle tarball members no longer use a `./` prefix (CI false failure)

## [0.6.16] - 2026-09-03

### Fixed

- CI LOC gate: split `emobie-inputd` socket RPC out of `main.rs` (was over 400 lines)
- AppImage launcher: prefer `GDK_BACKEND=x11` on Wayland to avoid blank/crashing WebKit (EGL_BAD_PARAMETER)

### Changed

- Ships the 0.6.15 Expand reliability work (hybrid key/clipboard insert) as a tagged release

## [0.6.15] - 2026-09-02

### Fixed

- Expand insert races on Plasma Wayland: key-safe ASCII types via uinput; clipboard restore off by default
- Clipboard path prefers `wl-copy`/`wl-paste`, then arboard, with Ctrl+V plus Shift+Insert
- AppImage bootstrap no longer overwrites a newer host `emobie-inputd` (semver/`--version`, not mtime-only)
- After Grant/restart, Expand re-syncs matches (disable → sync → enable)

### Changed

- Status/journal report insert backend and suppress job count; Settings toggle for clipboard restore
- Optional `eitype` / feature-flagged libei for Unicode; smoke treats unfocused empty fields as INFRA
- Docs: Plasma Grant, `wl-clipboard`, and clipboard-restore default

## [0.6.14] - 2026-09-02

### Fixed

- Wayland Expand inject prefers `/dev/uinput` (Plasma lacks virtual-keyboard); Enigo/XTest alone cannot reach native Wayland apps
- `can_inject` on Wayland requires writable uinput; Grant/verify/UI re-run when inject is missing
- AppShell category id typing after LOC extract (`number` vs `string`)

### Changed

- Docs (`LINUX.md`, `MACROS.md`): inject needs uinput via Grant on every package channel
- `verify-expand-setup.sh` fails on Wayland when uinput is missing/unwritable
- Release CI asserts AppImage host tarball includes udev rules + setup/policy
- Split oversized sources under the 400-line LOC budget (inject, listen, access, unix, App, Settings)
- Smoke checklist (`docs/SMOKE.md`) plus `npm run smoke:gate` / `smoke:expand` Expand E2E harness

## [0.6.13] - 2026-09-02

### Fixed

- Expand: cancel pending on edit/nav; modifiers no longer flush; re-buffer overlap char; restore trigger on disable / queue drop / inject fail
- Expand: completing-key release fires even under inject suppress; pending timeout; multi-char UTF-8 buffer push
- Clipboard restore uses a single burst original + epoch (no chained restores)
- Sync matches under lock then persist off the hot path; skip identical saves; sync before enable
- Socket peer stall: 5s client r/w timeouts; flock lock file forced to 0600
- Match caps reject NUL/control triggers; dedupe duplicate triggers on sync
- Inject worker: saturating suppress counts; clamp erase; recreate Enigo after failure; best-effort retype

### Changed

- XKB reload only on fingerprint change when no keys held; layout from env → kxkbrc → `/etc/default/keyboard` (`uk`→`gb`)
- Trusted socket parents must be uid-owned / not other-writable / sticky (Tauri client mirrored)
- `can_inject` TTL cache; longer inject suppress grace; hotplug poll 5s; longest-suffix match first

## [0.6.12] - 2026-09-02

### Fixed

- Expand Grant no longer skips Polkit when listen works via temporary ACL or orphaned GID
- AppImage/Flatpak Grant stages udev/policy siblings with the setup script under `/usr/local/share/emobie`
- `setup-input-access.sh` resolves rules from user bootstrap trees; recreates `emobie-input` idempotently; clearer immutable-/etc errors
- `setpriv` verification keeps supplementary groups (`--init-groups`)

### Changed

- Input helper status exposes `accessConfigured`; Settings/first-run show Repair when permanent access is incomplete
- `verify-expand-setup.sh` fails when `can_listen` is true but group/udev config is missing

## [0.6.11] - 2026-09-01

### Added

- Centralized input-helper IPC client; durable preference `writeRev` for multi-instance stale-write rejection
- Release CI verifies inputd assets in deb and rpm packages
- systemd hardening for `emobie-inputd` user units; threat-model docs in `MACROS.md`

### Changed

- Expand settings toggle delegates `set_enabled` to `useInputHelperSync`; YAML import size/count limits
- CSP drops `unsafe-inline` styles; skin-tone swatches use CSS classes
- Grant staging copies setup script to Polkit-annotated path before `pkexec`

### Fixed

- inputd: SetEnabled persist race, prefs resurrect after delete-all, listen buffer vs trigger length, pending-expand key handling
- inputd: paste inject suppress held until worker completes; keymap reload from session every ~30s
- Updates: bind `apply_update` to verified release tag; harden native `.deb` tar extraction
- AppImage autostart stable path; EXDEV copy fallback on in-place update
- Flatpak host `systemctl` via `flatpak-spawn`; serialized preference writes and IPC generation guards
- Helper sync errors surfaced in status bar; preference read failures shown in Settings

## [0.6.10] - 2026-09-01

### Added

- Favorite emoji macros: shortcodes and emoticons only for emojis in **Favorites**
- Settings → **Emoticon style** (`:) ` vs `:-) `)
- `emobie-inputd` bootstraps expand match rules from `preferences.json` when persisted state is empty (e.g. helper starts at login before the app)

### Changed

- Macros UI and docs: favorite-based emoji macros replace global built-in shortcode packs

### Fixed

- AppImage: stage WebKit network/web helper processes and fix gtk plugin library paths (reduces blank WebView on Wayland)
- AppImage icon staging: prefer larger root icon and set `.DirIcon`

## [0.6.9] - 2026-08-28

### Added

- Layout-aware trigger matching via libxkbcommon (follows session XKB layout)
- AppImage and Flatpak auto-install host `emobie-inputd` on first Expand (`inputd-host-bundle.tgz`)
- SELinux module auto-load during Grant when `checkmodule`/`semodule` are available

### Changed

- Expand flow for AppImage/Flatpak: enable Expand → helper installs automatically → one Grant prompt
- AppImage bundles input helper under `usr/share/emobie/`

## [0.6.8] - 2026-08-28

### Added

- `scripts/verify-expand-setup.sh` — diagnose helper, socket, keyboard access, compositor env, and SELinux hints across distros
- Optional SELinux module stub (`packaging/selinux/`) for Fedora/RHEL AVC denials
- Polkit action for `/usr/local/share/emobie/setup-input-access.sh` (AppImage / user helper installs)

### Fixed

- Text expansion inject on Plasma Wayland when `emobie-inputd` starts without `WAYLAND_DISPLAY` (auto-detect `$XDG_RUNTIME_DIR/wayland-0`)
- Do not force `DISPLAY=:0` on Wayland sessions (avoids duplicate keystrokes via XWayland + enigo)
- Grant/setup script: `modprobe uinput`, distro `acl` hints, access verification, helper restart, and `/usr/local` Polkit path for user installs
- `can_inject` checks compositor env and writable uinput instead of only path existence
- systemd user unit: `After=graphical-session.target`, compositor `PassEnvironment` (group access via udev/setfacl after Grant)
- Expand Grant status distinguishes listen vs inject readiness; Grant retry requires both before enabling

## [0.6.7] - 2026-08-28

### Fixed

- Native in-app updates install from the `.deb` into `~/.local/bin/emobie-bin` instead of an AppImage (avoids blank WebKit window on Wayland)
- AppImage launcher sets `WEBKIT_DISABLE_DMABUF_RENDERER` / `WEBKIT_DISABLE_COMPOSITING_MODE` to reduce blank-window issues
- Favorites, recents, and macros survive updates and Flatpak↔native switches via `~/.local/share/emobie/preferences.json` plus merge-on-load
- Taskbar icon on Wayland: GTK app id matches the desktop file (`io.github.asafelobotomy.emobie`)
- Linux packages install a full hicolor icon set under the app id (deb/rpm/AppImage/Flatpak/Arch/native)

## [0.6.6] - 2026-08-27

### Added

- Settings → Text expansion: optional **Keep Space after expansion** (`.hi` + Space → `hiya `)
- RPM packages ship the same `emobie-inputd` assets as `.deb`
- Flatpak: `xdg-data/emobie:ro` + host Grant path; KWin talk-name for Plasma pin
- Linux distro/DE guide ([`docs/LINUX.md`](docs/LINUX.md)); Arch PKGBUILD stub ([`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD))
- Flatpak launch-on-startup via XDG Background portal (desktop-file fallback when allowed)
- Settings hints for limited Wayland pin and GNOME AppIndicator tray
- In-app updater: download the matching GitHub release asset and install (deb/rpm/AppImage/Flatpak)
- Ubuntu + Ubuntu Mono as the app typeface (body, headings, brand, monospace triggers)

### Changed

- Text expansion controls live under Settings → Text expansion (not per-macro)
- Emoji shortcode macros stay collapsed unless searching or expanded
- Space-terminated expansion is the recommended default (e.g. `.hi` then Space)
- emobie-inputd starts with the app; enabling Expand as you type starts it if needed and turns listening on immediately
- Enabling Expand (or first-run setup) prompts once for keyboard access, restarts the helper, and skips logout when session ACLs apply
- Packaged setup uses `pkexec /usr/share/emobie/setup-input-access.sh` (matches Polkit policy)
- emobie-inputd listens on all keyboard devices and refreshes `can_listen` on each status query

### Fixed

- AppImage bundling on Arch/CachyOS: set `NO_STRIP` (and `APPIMAGE_EXTRACT_AND_RUN`) so linuxdeploy no longer fails on `.relr.dyn`
- Pin (always-on-top) on Plasma Wayland via KWin `keepAbove` (GTK keep-above is a no-op there)
- Flatpak Plasma pin calls host `qdbus` via `flatpak-spawn --host`
- Pin is re-applied after show/focus so it survives hide-to-tray
- Trailing spaces in expansions are typed as Space key events (more reliable than text inject)
- setfacl failures are reported instead of claiming ACLs always applied
- Flatpak Grant looks for the host-staged setup script via `flatpak-spawn --host`
- Flathub source manifest tag bumped to v0.6.6
- README notes updated for paste/expand and GNOME tray

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

[Unreleased]: https://github.com/asafelobotomy/emobie/compare/v0.6.17...HEAD
[0.6.17]: https://github.com/asafelobotomy/emobie/compare/v0.6.16...v0.6.17
[0.6.16]: https://github.com/asafelobotomy/emobie/compare/v0.6.15...v0.6.16
[0.6.15]: https://github.com/asafelobotomy/emobie/compare/v0.6.14...v0.6.15
[0.6.14]: https://github.com/asafelobotomy/emobie/compare/v0.6.13...v0.6.14
[0.6.13]: https://github.com/asafelobotomy/emobie/compare/v0.6.12...v0.6.13
[0.6.12]: https://github.com/asafelobotomy/emobie/compare/v0.6.11...v0.6.12
[0.6.11]: https://github.com/asafelobotomy/emobie/compare/v0.6.10...v0.6.11
[0.6.10]: https://github.com/asafelobotomy/emobie/compare/v0.6.9...v0.6.10
[0.6.9]: https://github.com/asafelobotomy/emobie/compare/v0.6.8...v0.6.9
[0.6.8]: https://github.com/asafelobotomy/emobie/compare/v0.6.7...v0.6.8
[0.6.7]: https://github.com/asafelobotomy/emobie/compare/v0.6.6...v0.6.7
[0.6.6]: https://github.com/asafelobotomy/emobie/compare/v0.6.5...v0.6.6
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
