# Linux distros, desktops, and permissions

How emobie behaves across popular Linux packaging formats and desktop
environments. Companion to [`MACROS.md`](MACROS.md) (input helper) and
[`FLATHUB.md`](FLATHUB.md) (Flatpak submission).

## Recommended install by distro

| Distro family | Prefer | Text expansion |
|---------------|--------|----------------|
| Ubuntu / Debian / Pop!_OS / Mint | `.deb` from [Releases](https://github.com/asafelobotomy/emobie/releases) | Bundled `emobie-inputd`; Expand → Polkit |
| Fedora / RHEL / openSUSE | `.rpm` | Same as `.deb` (helper + udev + polkit in package) |
| Arch / Manjaro / **CachyOS** | Flatpak or AppImage, or build from source | Auto host helper on Expand; fallback `install-inputd-user.sh` |
| Immutable (Silverblue, etc.) | Flatpak + host/user helper | Helper must run outside the app sandbox |

There is no official AUR package yet. A starting point lives in
[`packaging/arch/PKGBUILD`](../packaging/arch/PKGBUILD).

## Desktop environments

### Tray / taskbar

emobie uses **StatusNotifierItem** (`ksni`) on Linux.

| Desktop | What you need |
|---------|----------------|
| **KDE Plasma** | Built-in StatusNotifier — works out of the box |
| **Cinnamon / Mint** | Enable the **System Tray** applet; Mint uses `xapp-sn-watcher` |
| **GNOME** | Install an **AppIndicator / KStatusNotifierItem** Shell extension (e.g. [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/)). Without it, tray may fail and close will quit instead of hide |
| **XFCE / MATE** | Enable a StatusNotifier / AppIndicator panel plugin |
| **COSMIC / Hyprland / Sway** | Untested — tray and global hotkeys may be limited |

Settings shows a hint when the tray fails to start.

### Pin (always-on-top)

| Session | Behavior |
|---------|----------|
| **X11** (any DE) | GTK keep-above — reliable |
| **Plasma Wayland** | KWin `keepAbove` via `qdbus` (Flatpak calls **host** `qdbus6`/`qdbus`) |
| **Other Wayland** (GNOME, etc.) | Compositor may ignore pin; Settings warns when pin is limited |

### Launch on startup

| Install | Mechanism |
|---------|-----------|
| Native `.deb` / `.rpm` / AppImage / source | Writes `~/.config/autostart/emobie.desktop` |
| Flatpak (GitHub Releases) | Background portal when possible; falls back to `flatpak run` desktop file (`xdg-config/autostart:create`) |
| Flatpak (Flathub source) | **Background portal only** (no autostart filesystem permission) |

### Global hotkeys

Summon and macro hotkeys use the Tauri global-shortcut plugin. Behavior depends
on the compositor; some Wayland sessions restrict global shortcuts.

## Permissions (expand / auto-paste)

Text expansion needs three layers on every distro:

| Layer | What | How |
|-------|------|-----|
| **Helper daemon** | `emobie-inputd` on the **host** (same user as the desktop session) | Bundled in `.deb`/`.rpm`; AppImage/Flatpak auto-install on first Expand |
| **Keyboard read** | Open `/dev/input/event*` | udev group `emobie-input` + one-time **Grant** (Polkit); `setfacl` when `acl` package installed |
| **Text inject** | Type expansions into the focused app | Compositor env (`WAYLAND_DISPLAY` / `DISPLAY`); daemon auto-detects `$XDG_RUNTIME_DIR/wayland-0` |

**Verify your setup** (from a desktop terminal):

```bash
bash scripts/verify-expand-setup.sh
```

### Distro packages

| Distro | `acl` (setfacl) | Notes |
|--------|-----------------|-------|
| Ubuntu / Debian / Mint | `pkexec apt-get install acl` | Usually preinstalled |
| Fedora / RHEL | `pkexec dnf install acl` | See SELinux below |
| openSUSE | `pkexec zypper install acl` | |
| Arch / CachyOS | `pkexec pacman -S acl` | Expand auto-installs host helper |

### Grant (Polkit) paths

| Install | pkexec target |
|---------|---------------|
| `.deb` / `.rpm` | `/usr/share/emobie/setup-input-access.sh` |
| AppImage / user helper | `/usr/local/share/emobie/setup-input-access.sh` (created on first Grant) |
| Fallback | `pkexec /usr/local/share/emobie/setup-input-access.sh` |

1. **systemd --user** runs `emobie-inputd` (same user as the session — never root)
2. **udev** + group `emobie-input` (and optional **setfacl**) grant `/dev/input` read
3. **Polkit** prompts once for `setup-input-access.sh`

### SELinux (Fedora / RHEL)

Packaged `.rpm` installs standard paths under `/usr`. If Expand/Grant fails with
permission errors after a successful Polkit prompt:

```bash
# Confirm the helper can open input nodes after Grant + restart
systemctl --user status emobie-inputd.service
ls -la /dev/input/event* | head
# Optional: temporary permissive check while debugging (revert afterward)
# pkexec --keep-cwd setenforce 0
```

Report AVC denials with `ausearch -m avc -ts recent | grep emobie` and open an
issue if SELinux blocks the helper. Session `setfacl` usually avoids needing a
logout even when group membership is delayed. See `packaging/selinux/README.md`.

### Flatpak

The sandbox never gets `--device=input`. On first **Expand**, emobie installs
`emobie-inputd` on the host from the bundled archive, then runs Grant
(`flatpak-spawn --host pkexec …`).

Manual fallback (only if Expand auto-bootstrap fails):

```bash
bash packaging/install-inputd-user.sh
bash scripts/verify-expand-setup.sh
```

## Tray / taskbar icons

On Wayland, the compositor matches the window **app id** to a `.desktop` file
basename. emobie sets the GTK app id to `io.github.asafelobotomy.emobie` (same
as the Flatpak id and `Icon=` / `StartupWMClass=` in the desktop entry). If you
see a generic Wayland “W” icon after a local install, refresh icons and relaunch:

```bash
npm run icons:user
# or after a release build:
# bash scripts/install-user-icons.sh
update-desktop-database ~/.local/share/applications
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor
```

Quit and restart emobie after installing icons so Plasma picks up the new app id.

## Updates

With **Check for updates on startup** enabled, Settings shows a banner when a
newer [GitHub Release](https://github.com/asafelobotomy/emobie/releases) exists.

**Update now** downloads the asset that matches how you installed emobie:

| Detected install | Asset | Install path |
|------------------|-------|--------------|
| Flatpak | `.flatpak` | `flatpak install --user` (host via `flatpak-spawn` when sandboxed) |
| AppImage | `.AppImage` | Replaces `$APPIMAGE` in place |
| `.deb` | `.deb` | `pkexec apt-get install` / `dpkg -i` |
| `.rpm` | `.rpm` | `pkexec dnf` / `zypper` / `rpm -Uvh` |
| Native / other | `.deb` | Extracts `usr/bin/emobie` → `~/.local/bin/emobie-bin` |

Quit and relaunch after a successful update. Download URLs are limited to this
repo’s `releases/download/` assets.

Favorites, recents, and custom macros are stored in the app preference file and
mirrored to `~/.local/share/emobie/preferences.json` so they survive updates and
install-channel switches (native ↔ Flatpak). On startup emobie merges those
sources if one side is missing user data.

## Building AppImage locally

On Arch / CachyOS, raw `tauri build --bundles appimage` often fails with
`failed to run linuxdeploy` because linuxdeploy’s bundled `strip` cannot handle
modern libraries (`.relr.dyn`). Prefer:

```bash
npm run tauri:build
```

That sets `NO_STRIP` / `APPIMAGE_EXTRACT_AND_RUN` and always runs
`scripts/fix-linux-bundle-icons.sh` (WebKit helpers, desktop `Exec=`, host
inputd bundle). If you invoke `tauri build` directly, run the fix script after:

```bash
NO_STRIP=true APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri build -- --bundles appimage
bash scripts/fix-linux-bundle-icons.sh
```

## CachyOS notes

CachyOS is Arch-based. Typical setup:

1. Install Flatpak or AppImage (or build with `npm run tauri:build`)
2. Enable **Expand** in Settings (auto-installs the host helper + Grant); fallback:
   `bash packaging/install-inputd-user.sh`
3. On **Plasma** (common default): pin + tray + Grant are on the supported path

## Related docs

- [`MACROS.md`](MACROS.md) — macros layers and helper security
- [`FLATHUB.md`](FLATHUB.md) — Flathub finish-args and remaining gates
- [`README.md`](../README.md) — quick install table
