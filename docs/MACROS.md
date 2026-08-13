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
`enable --now`s it. emobie also calls `input_helper_ensure_started` on launch
(and when enabling Auto-paste / Expand) to start the unit or spawn a trusted
binary path if needed.

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

Daemon auto-start alone does **not** grant `/dev/input` access. One-time host
setup:

```bash
pkexec /usr/share/emobie/setup-input-access.sh
# or from a source checkout:
pkexec env SUDO_USER="$USER" bash packaging/setup-input-access.sh
```

Creates group `emobie-input`, installs udev rules, adds your user. **Log out
and back in.** Group membership is sensitive (keyboard event read access).

Expand-as-you-type stays **off by default** and only watches keys after you
enable it in Settings.

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
