# Macros

emobie macros let you store trigger → expansion snippets, browse them like
emoji, bind per-macro hotkeys, import/export Espanso-ish YAML, and optionally
auto-paste after copy via a host helper.

Settings → **Expand as you type** turns on trigger expansion: type a macro's
trigger anywhere (e.g. `.hi` then Space) and it is replaced by the expansion.
It is off by default and asks once for admin approval, because it needs the
helper to read your keyboard — see "Expand as you type" below.

## Using macros

Open the **Macros** category. Each card shows the expansion on top and the
trigger below — click to copy. Use **+** to add a custom macro; right-click a
custom card to edit or delete it.

Optional **favorite emoji macros** add shortcodes (`:smile:`) and emoticons for
emojis in your **Favorites** only. Choose **:) style** or **:-) style** under
Settings → Emoticon style. Right-click emojis in the grid to favorite them.
YAML import/export remains in Settings.

## Layers

| Layer | What | Flatpak |
|-------|------|---------|
| A | Macros UI, favorite emoji macros, YAML, hotkeys, clipboard copy | Fully supported |
| B | Auto-paste after copy (Ctrl+V + clipboard restore) | Needs host `emobie-inputd` |
| C | Expand as you type (trigger matching) | Needs host `emobie-inputd` + opt-in keyboard read |

Layers B and C use separate permissions: B only needs `/dev/uinput` **write**;
C additionally needs keyboard **read**, granted only when you turn it on and
removed with **Remove keyboard access**.

Flathub builds do **not** request `--device=input`. The UI talks to a socket at
`$XDG_RUNTIME_DIR/emobie/emobie-inputd.sock` when the helper is installed on the
host.

## Auto-start (recommended)

On first launch emobie opens a short setup dialog to start the input helper
and optionally grant paste access (Skip is fine). The helper runs as a
**systemd --user** service (same user as your desktop session — never root).

### User-local install (fallback)

AppImage and Flatpak install the host helper automatically when you enable
**Auto-paste on copy** (or finish first-run setup). Use the script only for
from-source builds or when auto-bootstrap fails:

```bash
bash packaging/install-inputd-user.sh
```

This builds `emobie-inputd` into `~/.local/bin` and installs a user unit.
emobie also calls `input_helper_ensure_started` on every launch so the helper
is up for paste. Enabling **Auto-paste on copy** grants paste-injection access
with one Polkit prompt when missing and restarts the helper.

### Distro / .deb package

The emobie `.deb` ships:

- `/usr/bin/emobie-inputd`
- `/usr/lib/systemd/user/emobie-inputd.service`
- `/usr/share/emobie/setup-input-access.sh` (udev/group setup)

After install:

```bash
systemctl --user enable --now emobie-inputd.service
```

### Socket security

- Directory: `$XDG_RUNTIME_DIR/emobie` mode `0700`
- Socket: `emobie-inputd.sock` mode `0600`
- Clients whose Unix peer UID ≠ daemon UID are rejected

## Paste access (Auto-paste on copy)

Daemon auto-start alone does **not** grant `/dev/uinput` access. Enabling
**Auto-paste on copy** (or first-run **Set up auto-paste**) runs a one-time
Polkit prompt that:

1. Creates group `emobie-input`, installs udev rules, and adds your user
2. Loads `uinput` if needed and grants `/dev/uinput` write (Wayland inject path)
3. Applies session ACLs with `setfacl` when available (no logout required)
4. Restarts `emobie-inputd` so it can inject immediately

This Grant requests no keyboard-**read** access — only write access to
`/dev/uinput`. Keyboard read is a separate opt-in for Expand as you type (see
"Expand as you type → Permissions").

On Wayland/Plasma, a synthetic Ctrl+V (kernel virtual keyboard via
`/dev/uinput`) pastes after copy. Clipboard content comes from `wl-copy` when
available, else arboard's native Wayland clipboard. Compositor “virtual
keyboard” protocols are often missing; X11/XTest alone does not reach native
Wayland apps. Grant's udev rule covers `uinput` for every package channel
(`.deb` / `.rpm` / Arch / AppImage / Flatpak host helper).

**Clipboard restore** after paste is **off by default** (Settings → Restore clipboard
after paste). Leaving it off avoids a common Plasma race where a delayed restore
wipes the next copy. `wl-clipboard` is **recommended** (deb/rpm `Recommends`,
Arch `optdepends`) — when present, paste verifies readiness against a separate
`wl-paste` process (a real compositor round trip) instead of only arboard's
in-process self-check, so install it for the most reliable Wayland paste.

Grant is **idempotent** and re-runs when permanent config is missing even if the
helper can already inject via a temporary ACL or an orphaned group id
(session `groups` shows a bare number instead of `emobie-input`).

Manual host setup (same script):

```bash
pkexec /usr/share/emobie/setup-input-access.sh
# or from a source checkout:
pkexec env SUDO_USER="$USER" bash packaging/setup-input-access.sh
```

Log out/in only if ACLs are unavailable, so new sessions inherit the group.
Group membership grants `/dev/uinput` write access (paste injection).

**Verify setup** from a desktop terminal:

```bash
bash scripts/verify-expand-setup.sh
```

Under Flatpak or AppImage, enabling Auto-paste stages the host helper
(`~/.local/bin/emobie-inputd`) and Grant runs host Polkit against
`setup-input-access.sh`. If Grant still fails, run on the host:

```bash
pkexec /usr/local/share/emobie/setup-input-access.sh
```

Only use `bash packaging/install-inputd-user.sh` as a fallback when auto-bootstrap
cannot find the bundled host tarball.

**Pin:** always-on-top uses GTK keep-above (works on X11) and, on Plasma
Wayland, KWin `keepAbove`. Other Wayland compositors may ignore pin.

Favorite emoji macros (when enabled) stay in **collapsed** sections on the Macros page.

## Build the helper manually

```bash
cargo build --release --manifest-path crates/emobie-inputd/Cargo.toml
```

```bash
./crates/emobie-inputd/target/release/emobie-inputd
```

Packaging assets:

- `packaging/systemd/emobie-inputd.service`
- `packaging/udev/99-emobie-input.rules`
- `packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy`
- `packaging/install-inputd-user.sh`
- `packaging/setup-input-access.sh`

## Security

Prefer the dedicated helper over granting the Flatpak sandbox raw input
devices. Do not run `emobie-inputd` as root or expose a world-writable socket.

### Threat model

| Boundary | Protection | Residual risk |
|----------|------------|---------------|
| Cross-user | Socket mode `0600`, directory `0700`, `SO_PEERCRED` rejects foreign UIDs on the daemon side; the app only connects to sockets in directories owned by you (or root, not group/other-writable) and owned by you itself | Stale sockets — prefer `$XDG_RUNTIME_DIR/emobie` |
| Same-user session | Any process running as **you** may call `InjectPaste`, `SyncMatches`, and `SetEnabled` on the Unix socket | Malware or a compromised app in your session can inject keystrokes — same trust as any input helper |
| Remote | No network listener; JSON line protocol on a local socket only | None without local code execution |
| Webview → helper | emobie talks to inputd via Tauri IPC; daemon enforces match/trigger caps | XSS in emobie could sync macros or request paste — treat the webview as trusted UI |

**Same-UID trust:** inputd is a session helper, not a privilege boundary against other
processes owned by your user. Do not run untrusted binaries alongside emobie
when relying on Auto-paste.

**Polkit / root:** paste access setup runs once via `pkexec`, and only ever runs
root-owned inputs: either the package's `/usr/share/emobie/setup-input-access.sh`
or a copy staged to `/usr/local/share/emobie/`. The staged script, udev rule,
polkit policy and SELinux module are the exact bytes **embedded in the emobie
binary at build time** (never read from `~/.local/share/emobie`, the AppImage
mount, or any other user-writable path), and the root script refuses to run if
it or its directory is not root-owned. Re-running Grant also replaces an
outdated installed udev rule; the app treats a rule that differs from the
shipped one as "not configured".

## Expand as you type

### How it works

`emobie-inputd` reads keyboard events (`/dev/input/event*`) while expansion is
on, maps them to text with libxkbcommon using your **session layout**, and
matches triggers. On a match it erases the trigger with Backspace and inserts
the expansion through its `/dev/uinput` virtual keyboard:

- **Typed as keys** whenever your layout can produce every character (up to
  160 characters, no newlines/tabs): planned against the active layout, with
  Shift/AltGr and Caps Lock honoured. No clipboard and no paste shortcut, so it
  works the same in terminals, Kate, browsers and games.
- **Pasted** only for text the layout cannot type (emoji, other scripts,
  multi-line): clipboard + a per-app paste chord (see below). An optional
  system `eitype` (libei) is tried first.

The layout comes from `XKB_DEFAULT_*`, GNOME input sources (following the
active source live via `gsettings monitor`), Plasma `kxkbrc` (following the
active layout via `org.kde.keyboard`), `localectl`'s
`/etc/X11/xorg.conf.d/00-keyboard.conf`, then `/etc/default/keyboard` /
`/etc/vconsole.conf`.

The typed-text buffer resets on mouse/touch presses, Ctrl/Alt/Super
shortcuts, arrows, Enter, Tab and Esc, so a trigger split across a caret move
never fires. Keyboards behind remappers (keyd, kanata, kmonad) work: their
virtual output device is read; only emobie's own injector is skipped.

### Permissions

Turning it on runs Grant with `--keyboard-read on`, which installs
`/etc/udev/rules.d/98-emobie-keyboard-read.rules`. The rule adds a read
**ACL** for group `emobie-input` on keyboards, mice, touchpads and
touchscreens — device ownership is untouched, so the `input` group (used by
other tools) keeps its access. A user ACL is applied immediately so no
re-login is needed. Needs the `acl` package (`setfacl`). **Remove keyboard
access** runs `--keyboard-read off`, deleting the rule and both ACLs. On
SELinux the module allows reading `event_device_t`.

### Where it stays quiet

- **Lock screen:** matching pauses while logind reports the session locked
  (`LockedHint`), so an unlock password can never trigger an expansion.
- **Excluded apps** (Settings; defaults cover common password managers and
  authentication prompts): matched against the focused app's class. Detection
  works for X11/XWayland apps everywhere and for native Wayland apps on GNOME
  with the Focused Window D-Bus extension; native Plasma Wayland apps cannot
  be identified yet, so the list does not apply to them.
- **Suspend/resume:** input devices are reopened in place on resume
  (logind `PrepareForSleep`); the helper no longer restarts itself.

### Environments

| Session | Status |
|---------|--------|
| X11 (any desktop) | Supported |
| GNOME Wayland | Supported; layout switching followed live |
| Plasma Wayland | Supported; layout switching followed live; excluded apps not detectable |
| Sway / Hyprland / COSMIC / other Wayland | Supported via evdev + uinput; layout from `XKB_DEFAULT_*` / `localectl` |
| Flatpak | Supported through the host helper (sandbox never gets `--device=input`) |
| Immutable distros | Needs writable `/etc/udev/rules.d` for Grant |

## Known limitations

- **Password fields can't be detected.** Reading the keyboard directly (as
  Espanso does on Wayland) cannot tell a password box from any other; the
  lock-screen pause and the excluded-apps list are the mitigations.
- **Paste chord for untypeable text.** Emoji and multi-line expansions still
  paste. The chord is focused-window-aware ([`paste_chord.rs`](../crates/emobie-inputd/src/paste_chord.rs)):
  X11/XWayland via `_NET_ACTIVE_WINDOW` + `WM_CLASS`, GNOME Wayland via the
  optional ["Focused Window D-Bus"](https://extensions.gnome.org/extension/5592/)
  extension, otherwise Ctrl+V. Known terminals get Ctrl+Shift+V; apps like
  Kate that bind it to something else never do. **Settings → Clipboard →
  Paste key** forces a chord.
- **Dead-key / Compose characters** (e.g. `é` on US-International) are not
  typed as keys; they fall back to paste.
- **Layouts on wlroots compositors** are not read from the compositor; set
  `XKB_DEFAULT_LAYOUT` for the user session (or `localectl set-x11-keymap`)
  if yours differs from the system default.
- **Global hotkeys** (summon, per-macro) use X11 grabs, so on Wayland they
  only fire while an XWayland app is focused.

## YAML format

```yaml
matches:
  - trigger: ":sig"
    replace: |
      Best regards
  - trigger: ":ship"
    replace: "🚀"
    hotkey: Control+Alt+S   # emobie extension; ignored by Espanso
```
