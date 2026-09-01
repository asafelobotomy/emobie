# Macros and text expansion

emobie macros let you store trigger → expansion snippets, browse them like
emoji, bind per-macro hotkeys, import/export Espanso-ish YAML, and optionally
expand as you type via a host helper.

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
| C | As-you-type trigger expansion | Needs host `emobie-inputd` with input access |

Flathub builds do **not** request `--device=input`. The UI talks to a socket at
`$XDG_RUNTIME_DIR/emobie/emobie-inputd.sock` when the helper is installed on the
host.

## Auto-start (recommended)

On first launch emobie opens a short setup dialog to start the input helper
and optionally grant keyboard access (Skip is fine). The helper runs as a
**systemd --user** service (same user as your desktop session — never root).

### User-local install (fallback)

AppImage and Flatpak install the host helper automatically when you enable
**Expand** (or finish first-run setup). Use the script only for from-source
builds or when auto-bootstrap fails:

```bash
bash packaging/install-inputd-user.sh
```

This builds `emobie-inputd` into `~/.local/bin` and installs a user unit.
emobie also calls `input_helper_ensure_started` on every launch so the helper
is up for paste. Enabling **Expand as you type** grants keyboard access with
one Polkit prompt when missing, restarts the helper, and turns on listening.

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

## Keyboard access (expand as you type)

Daemon auto-start alone does **not** grant `/dev/input` access. Enabling
**Expand as you type** (or first-run **Set up text expansion**) runs a one-time
Polkit prompt that:

1. Creates group `emobie-input`, installs udev rules, and adds your user
2. Applies session ACLs with `setfacl` when available (no logout required)
3. Restarts `emobie-inputd` so it can open keyboards immediately

Manual host setup (same script):

```bash
pkexec /usr/share/emobie/setup-input-access.sh
# or from a source checkout:
pkexec env SUDO_USER="$USER" bash packaging/setup-input-access.sh
```

Log out/in only if ACLs are unavailable, so new sessions inherit the group.
Group membership is sensitive (keyboard event read access).

**Verify setup** from a desktop terminal:

```bash
bash scripts/verify-expand-setup.sh
```

Under Flatpak or AppImage, enabling Expand stages the host helper
(`~/.local/bin/emobie-inputd`) and Grant runs host Polkit against
`setup-input-access.sh`. If Grant still fails, run on the host:

```bash
pkexec /usr/local/share/emobie/setup-input-access.sh
```

Only use `bash packaging/install-inputd-user.sh` as a fallback when auto-bootstrap
cannot find the bundled host tarball.

**Layout note:** trigger matching follows your active XKB layout
(`XKB_DEFAULT_*` / session keyboard settings). Each keyboard listener reloads
the layout from session env every ~30 seconds. Modifier state is per device;
IME compose sequences are not supported.

**Pin:** always-on-top uses GTK keep-above (works on X11) and, on Plasma
Wayland, KWin `keepAbove`. Other Wayland compositors may ignore pin.

Expand-as-you-type stays **off by default**. Enable it under
**Settings → Text expansion**, and choose **After Space** to expand only when
you finish a trigger with Space (for example type `.hi` then Space). Optionally
enable **Keep Space after expansion** so `.hi` + Space becomes `hiya `
(with a trailing space) instead of `hiya`.

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
| Cross-user | Socket mode `0600`, directory `0700`, `SO_PEERCRED` rejects foreign UIDs | Misconfigured `/tmp/emobie-$UID` or stale sockets — prefer `$XDG_RUNTIME_DIR/emobie` |
| Same-user session | Any process running as **you** may call `InjectPaste`, `SyncMatches`, and `SetEnabled` on the Unix socket | Malware or a compromised app in your session can inject keystrokes — same trust as any input helper |
| Remote | No network listener; JSON line protocol on a local socket only | None without local code execution |
| Webview → helper | emobie talks to inputd via Tauri IPC; daemon enforces match/trigger caps | XSS in emobie could sync macros or request paste — treat the webview as trusted UI |

**Same-UID trust:** inputd is a session helper, not a privilege boundary against other
processes owned by your user. Do not run untrusted binaries alongside Expand when
you rely on as-you-type expansion.

**Polkit / root:** keyboard access setup runs once via `pkexec` on annotated script
paths only (`/usr/share/emobie/…` or `/usr/local/share/emobie/…`). User-writable
copies are staged to `/usr/local/share/emobie/` before elevation.

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
