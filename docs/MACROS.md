# Macros and text expansion

Emobie macros let you store trigger → expansion snippets, browse them like
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

## Build the helper

```bash
cargo build --release --manifest-path crates/emobie-inputd/Cargo.toml
```

Run (user session):

```bash
./crates/emobie-inputd/target/release/emobie-inputd
```

Packaging assets:

- `packaging/systemd/emobie-inputd.service`
- `packaging/udev/99-emobie-input.rules`
- `packaging/polkit/io.github.asafelobotomy.Emobie.inputd.policy`

Create group `emobie-input`, install udev rules, add your user to the group,
log out/in, then enable the user unit.

## Security

Expand-as-you-type watches keyboard events to match triggers. It is **off by
default** and requires an explicit Settings toggle. Prefer the dedicated helper
binary over granting the Flatpak sandbox raw input devices.

## YAML format

```yaml
matches:
  - trigger: ":sig"
    replace: |
      Best regards
  - trigger: ":ship"
    replace: "🚀"
    hotkey: Control+Alt+S   # Emobie extension; ignored by Espanso
```
