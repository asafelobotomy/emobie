# Macros

emobie macros let you store trigger → expansion snippets, browse them like
emoji, bind per-macro hotkeys, import/export Espanso-ish YAML, and optionally
auto-paste after copy via a host helper.

> **As-you-type text expansion is deferred for now.** Macros still work as a
> browsable, copyable (and optionally auto-pasted) snippet library — the
> keyboard-listening/trigger-matching half described in older versions of this
> doc (Layer C below) is disabled. The paste-chord bug that motivated the
> deferral now has a real (if incomplete — see "Known limitations") fix for
> Auto-paste; re-enabling Layer C is a separate decision, not blocked on that
> fix anymore. The daemon, its protocol, and the udev/polkit/SELinux plumbing
> for keyboard listening are still in the tree and can be re-enabled later;
> this doc describes the current (paste-only) behavior.

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
| C | As-you-type trigger expansion | **Deferred** — see note above |

Only Layers A and B are active. Layer C's daemon-side code (keyboard listening,
trigger matching) still exists but the app never enables it, and the packaged
udev rule / SELinux module / Polkit setup no longer request keyboard **read**
access — only `/dev/uinput` **write** access for Layer B's paste injection.

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

No keyboard-**read** access is requested — as-you-type text expansion is
deferred (see "Known limitations"), so Grant only needs the write-only uinput
half, a meaningfully smaller permission than reading every keystroke.

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

## Known limitations

- **As-you-type text expansion is deferred.** See the note at the top of this
  doc — Layer C (trigger listening) is disabled and has no Settings UI right
  now, pending a fix for the paste-chord issue below.
- **Paste chord is now focused-window-aware, with real coverage gaps.**
  0.6.19 fixed Kate (and other apps that bind both Ctrl+V and Shift+Insert to
  paste) double-pasting by dropping Shift+Insert, which broke terminals that
  need Ctrl+Shift+V instead (Ctrl+V is claimed by the shell there — confirmed
  live: it's a no-op in GNOME Console). No single fixed chord works for every
  app — adding Shift+Insert back reintroduces the Kate double-paste, and
  adding Ctrl+Shift+V instead silently triggers Kate's own "Switch to Next
  Input Mode" shortcut. [`paste_chord.rs`](../crates/emobie-inputd/src/paste_chord.rs)
  now picks the chord from the focused app's WM_CLASS via
  [`focused_window`](../crates/emobie-inputd/src/focused_window), matching
  the same approach Espanso (the closest prior art) uses:
  - **X11 / XWayland**: `_NET_ACTIVE_WINDOW` + `WM_CLASS`, works on plain X11
    sessions and XWayland-backed apps under Wayland.
  - **GNOME Wayland**: the optional, community-maintained
    ["Focused Window D-Bus"](https://extensions.gnome.org/extension/5592/)
    Shell extension, when installed. Not bundled — Espanso's own
    app-detection is explicitly unsupported on Wayland without an equivalent,
    and there is no built-in GNOME API for this (Shell's `Eval` is locked
    outside dev mode).
  - **Neither present** (native-Wayland toolkit apps with no extension
    installed — e.g. plain GNOME/KDE Wayland without the extension): falls
    back to the pre-existing Ctrl+V default, unchanged from before this file
    existed.
  - The known-terminal list in `paste_chord.rs` is a curated compatibility
    table (same approach Espanso's hard-coded per-app patches use), not a
    generic rule — if a terminal you use isn't recognized, add it there, or
    use **Settings → Clipboard → Paste key** to force a chord manually.
- **The daemon only reads the keyboard while expansion is enabled.** The
  listener starts on `SetEnabled(true)` (or at boot if the persisted state says
  enabled) and closes its keyboard devices shortly after it is disabled again;
  a paste-only daemon never holds `/dev/input/event*` open. If you granted
  access with an older release, your installed
  `/etc/udev/rules.d/99-emobie-input.rules` may still contain a keyboard-read
  rule — the app now reports that as "outdated"; re-run **Grant** to replace it.
- **Typed-key expansion assumes a US-QWERTY layout.** The short-ASCII fast path
  sends physical keycodes, which the compositor maps through your active
  layout, so on AZERTY/QWERTZ the text comes out wrong. It is only reachable
  through the deferred Expand feature; fix (route non-US layouts through the
  clipboard path) before re-enabling it.
- **The daemon's trigger-listening code is otherwise dormant.** Now that
  as-you-type expansion is deferred, a compositor crash/restart affecting the
  (unused) listen thread is no longer user-visible — noted here only because
  [`sleep_watch.rs`](../crates/emobie-inputd/src/sleep_watch.rs) and the
  listen/matcher code paths still exist in the tree for when expansion
  returns, and this class of bug will need re-checking then.

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
