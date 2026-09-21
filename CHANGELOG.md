# Changelog

All notable changes to emobie are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- **Privileged setup no longer trusts user-writable files.** Grant used to copy
  a `setup-input-access.sh` (newest mtime wins), udev rule and polkit policy from
  `~/.local/share/emobie/` into `/usr/local` and run/install them as root, so any
  process running as the user could plant a root payload. The app now stages the
  exact bytes embedded in its own binary, and the root script refuses to run
  unless it and its directory are root-owned and no longer reads user homes.
- **Updater verifies downloads.** One-click update requires a `SHA256SUMS`
  release asset, streams the download into a private `0700` directory, checks the
  hash, then re-verifies it on a root-owned copy right before `dpkg`/`dnf`/`rpm`
  runs (closes the swap-after-download window). It also refuses non-newer
  versions and malformed tags. The release workflow now publishes `SHA256SUMS`.
- **`emobie-inputd` reads the keyboard only while expansion is enabled.** It used
  to hold a keyboard event device open unconditionally. The app now also treats an
  installed udev rule that differs from the shipped one as "not configured" —
  installs from before the keyboard-read rule was removed keep it until Grant is
  re-run.
- The app only connects to inputd sockets in directories owned by you (or root),
  owned by you themselves; the same rule applies inside the daemon.
- `preferences.json` is written `0600` (atomic, fsynced, unique temp file).

### Fixed

- **Host-helper bootstrap and native (`~/.local`) updates were broken since
  0.6.11**: `tar --no-absolute-names` is not a GNU tar option, so extraction always
  failed. The release Flatpak's host bundle also had `./`-prefixed members that the
  extractor could not match. Both fixed, with regression tests that run real `tar`.
- The "Set up GNOME pin shortcut" button was rejected by the Tauri ACL
  (`pin_gnome_setup` had no permission). A new `check:acl` script fails CI if a
  registered or invoked command lacks a permission.
- GNOME pin: overlapping toggles could double-toggle (tray + startup) — now
  serialized; an unknown state no longer sends the *unpin* chord (which pinned a
  fresh window); the toggle waits for emobie to be focused; the summon hotkey
  focuses before pinning; `apply_window_pin` no longer blocks the main thread.
- Preferences: deleting a macro/favorite/recent (or clearing recents/usage stats)
  no longer comes back after a restart — the active store is authoritative and
  other snapshots only recover keys it lacks. The durable mirror's write revision is
  now monotonic across launches (it used to restart at 1 and silently drop writes).
- systemd unit no longer fails with 226/NAMESPACE when `~/.local/share/emobie`
  does not exist yet (`ReadWritePaths=-…`).
- The root setup script no longer aborts with "same file" when run from its
  staged `/usr/local` copy.
- Paste no longer fires late after the app already reported a timeout, and the
  focused-window lookup is bounded (300 ms) instead of able to stall every paste.
- Rust command errors (plain strings) are now shown instead of a generic message.
- Input helper commands (`input_helper_*`) run on the blocking pool instead of the
  UI thread (the first-run hook polls status every 15 s, and each poll can spawn
  host processes), and helper start/restart are serialized so concurrent callers
  cannot double-bootstrap.
- Held keys are tracked as a set, so autorepeat no longer leaves the keymap
  reload permanently blocked; a panicking keyboard-device thread now frees its
  slot for retry; `/etc/default/keyboard` parsing no longer aborts on a line
  without `=`.
- `.deb` icon repack regenerates `md5sums`; `verify-expand-setup.sh` warns about an
  installed udev rule that still grants keyboard read; the hotkey capture control
  no longer reuses one DOM id across instances.
- Macro import skips triggers/expansions the helper would reject (control
  characters, NUL) instead of failing the whole sync; autostart `Exec=` is quoted
  per the Desktop Entry spec; the suspend/resume restart only happens under systemd.

### Changed

- Flatpak offline sources now cover both lockfiles (`scripts/flatpak-cargo-sources.py`, plain Python 3 — no `tomllib`, so it runs on the CI runner's 3.10).
- CI: actions pinned to commit SHAs, least-privilege workflow permissions,
  `appimagetool` pinned to 1.9.1 with a checksum, clippy (`-D warnings`), app
  crate tests, production `npm audit`, Flatpak-sources freshness and ACL checks.
- `PKGBUILD` bumped to 0.6.25; README Node requirement corrected to 22.6+.

## [0.6.25] - 2026-09-16

### Fixed

- **Fixed startup freeze on GNOME Wayland with Pin enabled.** Applying the
  saved Pin state ran synchronously inside Tauri's `setup()` callback — the
  main thread, before the window's event loop was pumping. On GNOME Wayland
  this call chain (`pin::apply_from_prefs` → `ensure_started()`) can shell
  out to bootstrap/install `emobie-inputd`, run `systemctl --user`, and
  poll every 150ms for up to ~10 seconds total, which froze the whole
  window on every launch whenever the daemon wasn't already running. Pin
  application now runs on a background thread so the window shows and
  becomes interactive immediately (`src-tauri/src/lib.rs`).
- Capped `ensure_started()`'s bootstrap/start chain at 12 seconds
  (`src-tauri/src/input_helper/unix/lifecycle.rs`). The chain shells out to
  tar/bash/systemctl (and `flatpak-spawn --host` under Flatpak) with no
  per-call timeout of their own, so a wedged host command (e.g. a stuck
  portal prompt) could otherwise hang callers indefinitely even off the UI
  thread.

## [0.6.24] - 2026-09-12

### Added

- Pin now works on GNOME Wayland. Mutter has no external always-on-top API
  (`make_above`/`unmake_above` are only callable from inside the Shell
  process), but it does ship an unbound-by-default `toggle-above`
  keybinding. Settings → "Set up GNOME pin shortcut" claims that binding
  for a fixed chord (Ctrl+Alt+Super+F12, never overwriting an existing
  shortcut), and `emobie-inputd` synthesizes that exact keypress via
  `/dev/uinput` when you toggle Pin — no custom GNOME Shell extension
  required. Verified live: the exact synthetic chord correctly dispatches
  GNOME keybinding actions (confirmed via `OverviewActive` flipping on a
  temporary rebind test).
- Since `toggle-above` toggles rather than sets state, `src-tauri/src/pin/linux/gnome.rs`
  tracks emobie's own last-known above/below state and only sends a
  keypress on a real transition, resetting to "unknown" on hide so a
  stale tracked state can never suppress a needed re-assertion after
  show. Window focus is now grabbed *before* re-applying pin on show
  (previously focus was requested after — harmless for the existing X11/
  KWin paths, but would have misdirected the new synthetic keypress).

## [0.6.23] - 2026-09-12

### Added

- Auto-paste now picks its paste chord based on the focused app instead of
  always sending Ctrl+V. Detection is best-effort and layered: X11/XWayland
  via `_NET_ACTIVE_WINDOW`+`WM_CLASS` (works on plain X11 sessions and
  XWayland-backed apps under Wayland — verified live), or the optional,
  community-maintained ["Focused Window D-Bus"](https://extensions.gnome.org/extension/5592/)
  GNOME Shell extension for native-Wayland GNOME apps. Falls back to the
  previous Ctrl+V default when neither is available. A curated list of known
  terminal emulators gets Ctrl+Shift+V instead (Ctrl+V is claimed by the
  shell there); apps like Kate that bind Ctrl+Shift+V to something else
  (KDE's "Switch to Next Input Mode") are explicitly excluded from ever
  receiving it. See `crates/emobie-inputd/src/paste_chord.rs` and
  `focused_window/`.
- New **Settings → Clipboard → Paste key** override (Auto-detect / always
  Ctrl+V / always Shift+Insert / always Ctrl+Shift+V) for apps the built-in
  detection doesn't recognize.

## [0.6.22] - 2026-09-12

### Changed

- **As-you-type text expansion is deferred.** The "Expand as you type" toggle
  and its trigger-listening code are no longer reachable from the UI, pending
  a real fix for the paste-chord bugs affecting it (Ctrl+V is a no-op in some
  terminals; the alternatives reintroduce the Kate double-paste bug or
  trigger an unrelated Kate shortcut — see docs/MACROS.md "Known
  limitations"). Macros still work fully for browsing, copying, and
  optional auto-paste-on-copy. The daemon-side listen/matcher code and
  protocol are unchanged in the tree for a future re-enable.
- Packaged udev rules, the Polkit action, and the optional SELinux module no
  longer request keyboard **read** access (`/dev/input/event*`) — only
  `/dev/uinput` **write** access for paste injection, a meaningfully smaller
  permission grant. Existing installs with the old rule still work; new
  installs no longer prompt for or receive keyboard-read access.

## [0.6.21] - 2026-09-10

### Fixed

- Flatpak: copying (emoji picker, "Copy" button) could fail outright on
  Wayland-native sessions (GNOME/Mutter, etc.) with a "Copy Failed" toast.
  `finish-args` used `--socket=fallback-x11`, which only grants X11 access
  when Wayland is *unavailable* — on a real Wayland session this left
  `DISPLAY` unset inside the sandbox, and the direct copy path (via
  `tauri-plugin-clipboard-manager`'s `arboard`, built without the
  `wayland-data-control` feature) had no X11 to fall back to and no native
  Wayland path either. Switched to `--socket=x11` (always available,
  regardless of session type) across all three Flatpak manifests
- Flatpak/Expand: the main app's direct clipboard-copy path used the
  XWayland/X11 clipboard bridge unconditionally (arboard's default Linux
  backend), unlike `emobie-inputd`'s paste path which already got the
  0.6.18 native-Wayland-clipboard fix. Added `arboard`'s
  `wayland-data-control` feature as a direct dependency of the main app too,
  so Cargo's feature unification gives `tauri-plugin-clipboard-manager` the
  same native Wayland clipboard behavior instead of relying on XWayland

## [0.6.20] - 2026-09-06

### Added

- Macro editor: a Bold/Italic/Strikethrough/Code/Quote toolbar that wraps the
  selected expansion text in standard Markdown-style markers
  (`**bold**`, `*italic*`, `~~strike~~`, `` `code` ``, `> quote`)

### Fixed

- Expand: text expansion could silently stop working after long idle periods
  (observed: worked after boot, broke after several hours) and restarting the
  app never fixed it — only `systemctl --user restart emobie-inputd` did.
  Root cause: the persistent `/dev/uinput` virtual device is opened once and
  reused for the daemon's whole life; a long-idle virtual device can go stale
  on the compositor side (same class of bug already worked around for Enigo
  via `ENIGO_MAX_IDLE`) while writes keep succeeding at the kernel level, so
  no error ever fires the existing recovery path — and the app's own health
  check only re-bootstraps the daemon when the socket stops responding at
  all, which a wedged-but-alive daemon never does. Now mirrors the Enigo
  idle-recreate policy for uinput (reopen after 5 minutes idle) and fixes a
  second gap where a failed Ctrl+V paste job permanently dropped uinput with
  no retry (unlike the equivalent Expand-job failure path, which already
  retried). Both fixes are purely on-demand — checked only when a job
  actually runs, no background polling or periodic restarts added
- Expand: confirmed via live diagnosis (thread state + kernel suspend logs)
  that a genuine suspend/resume cycle leaves the evdev keyboard-read thread
  parked forever with zero events delivered and no I/O error — trigger
  detection itself silently dies, distinct from the uinput-staleness fix
  above. Added [sleep_watch.rs](crates/emobie-inputd/src/sleep_watch.rs):
  an event-driven (not polled) subscription to logind's `PrepareForSleep`
  D-Bus signal that restarts the process on resume, the same fix
  `systemctl --user restart emobie-inputd` already provided manually, now
  applied automatically at the exact moment it's needed

## [0.6.19] - 2026-09-03

### Fixed

- Expand: the uinput paste path sent Ctrl+V *and* Shift+Insert unconditionally on
  every expansion, intended as a fallback for terminals that don't bind Ctrl+V. Any
  app that binds both (Kate, and most Qt/KDE apps do) pasted the expansion twice —
  visible as the whole macro duplicated back-to-back. Send Ctrl+V only, matching the
  X11/Enigo path, which never sent the second chord

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
