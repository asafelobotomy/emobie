# Macros and text expansion

emobie macros let you store trigger → expansion snippets, browse them like
emoji, bind per-macro hotkeys, import/export Espanso-ish YAML, and optionally
expand as you type via a host helper.

## Using macros

Open the **Macros** category. Each card shows the expansion on top and the
trigger below — click to copy. Use **+** to add a custom macro; right-click a
custom card to edit or delete it.

Built-in emoji macros include shortcodes (`:smile:`) from common packs plus
ASCII emoticons (`:)`, `;')`, `:')`, `<3`, …). Toggle them in Settings →
Show emoji shortcodes. YAML import/export remains in Settings.

## Layers

| Layer | What | Flatpak |
|-------|------|---------|
| A | Macros UI, shortcodes, YAML, hotkeys, clipboard copy | Fully supported |
| B | Auto-paste after copy (Ctrl+V + clipboard restore) | Needs host `emobie-inputd` |
| C | As-you-type trigger expansion | Needs host `emobie-inputd` with input access |

Flathub builds do **not** request `--device=input`. The UI talks to a socket at
`$XDG_RUNTIME_DIR/emobie/emobie-inputd.sock` when the helper is installed on the
host.

## Auto-start (recommended)

On first launch emobie opens a short setup dialog to start the input helper
and optionally grant keyboard access (Skip is fine). The helper runs as a
**systemd --user** service (same user as your desktop session — never root).

### User-local install

```bash
bash packaging/install-inputd-user.sh
```

This builds `emobie-inputd` into `~/.local/bin`, installs a user unit, and
emobie also calls `input_helper_ensure_started` on every launch (systemd
`enable --now` or a trusted local binary) so the helper is up for paste and
ready when you turn on expansion. Enabling **Expand as you type** starts the
helper if needed, grants keyboard access with one Polkit prompt when missing,
restarts the helper, and turns on keystroke listening immediately.

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

Under Flatpak, install the host helper first
(`bash packaging/install-inputd-user.sh`), which stages
`~/.local/share/emobie/setup-input-access.sh`. Expand/Grant then runs
`flatpak-spawn --host pkexec …` against that host script. If Grant still fails,
run the host `pkexec` command above and retry.

**Layout note:** as-you-type matching maps keys assuming a US QWERTY physical
layout. Triggers that need other layouts may not fire until mapping improves.

**Pin:** always-on-top uses GTK keep-above (works on X11) and, on Plasma
Wayland, KWin `keepAbove`. Other Wayland compositors may ignore pin.

Expand-as-you-type stays **off by default**. Enable it under
**Settings → Text expansion**, and choose **After Space** to expand only when
you finish a trigger with Space (for example type `.hi` then Space). Optionally
enable **Keep Space after expansion** so `.hi` + Space becomes `hiya `
(with a trailing space) instead of `hiya`.

Built-in emoji shortcodes stay in **collapsed** sections on the Macros page.

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
