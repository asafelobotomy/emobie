<p align="center">
  <img src="branding/emobie-icon-transparent.png" alt="emobie" width="128" height="128" />
</p>

<h1 align="center">emobie</h1>

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/U5R225QZH3)

<p align="center">
  <strong>Compact Linux-first emoji palette for content creators</strong>
</p>

<p align="center">
  Browse the full Unicode set · search · copy · pin · tray · global hotkey
</p>

<p align="center">
  <a href="https://github.com/asafelobotomy/emobie/releases/latest"><img src="https://img.shields.io/github/v/release/asafelobotomy/emobie?style=flat-square&label=release&color=1a6b5a" alt="Latest release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-3dba9a?style=flat-square" alt="MIT License" /></a>
  <a href="https://github.com/asafelobotomy/emobie/releases"><img src="https://img.shields.io/badge/platform-Linux-1a6b5a?style=flat-square" alt="Linux" /></a>
  <a href="https://github.com/asafelobotomy/emobie/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/asafelobotomy/emobie/ci.yml?branch=main&style=flat-square&label=CI" alt="CI" /></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#features">Features</a> ·
  <a href="#develop">Develop</a> ·
  <a href="#build--package">Build</a> ·
  <a href="CHANGELOG.md">Changelog</a>
</p>

---

## Install

Grab the latest build from
[**GitHub Releases**](https://github.com/asafelobotomy/emobie/releases/latest).

| Format | Best for | How |
|--------|----------|-----|
| **Flatpak** | Sandboxed desktop install | `flatpak install --user emobie-*-x86_64.flatpak` — Expand auto-installs the host helper |
| **`.deb`** | Debian / Ubuntu / Mint / Pop | `sudo apt install ./emobie_*_amd64.deb` |
| **`.rpm`** | Fedora / RHEL / openSUSE | `sudo dnf install ./emobie-*-1.x86_64.rpm` (openSUSE: `zypper install …`) |
| **AppImage** | Portable / Arch / CachyOS | `chmod +x emobie_*.AppImage && ./emobie_*.AppImage` — Expand auto-installs the host helper |

Arch / CachyOS / Manjaro: see [`docs/LINUX.md`](docs/LINUX.md) and optional [`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD).

```bash
# Flatpak (after downloading the release bundle)
flatpak install --user emobie-*-x86_64.flatpak
flatpak run io.github.asafelobotomy.emobie
```

App ID: `io.github.asafelobotomy.emobie`

Linux tray, pin, startup, and SELinux notes: [`docs/LINUX.md`](docs/LINUX.md).

---

## Features

| | |
|---|---|
| **Catalog** | Full Unicode emoji set via emojibase, with categories and search (name, tags, shortcodes) |
| **Favorites & recents** | Right-click to favorite; recent history with configurable size |
| **Macros** | Custom trigger → expansion cards (+ to add), optional favorited-emoji shortcodes/emoticons, per-macro hotkeys, YAML import/export |
| **Updates** | Optional startup check; Settings can download and install the matching release asset (deb/rpm/AppImage/Flatpak) |
| **Copy** | One-click clipboard copy; optional auto-paste when the host input helper is available |
| **Expand** | Optional as-you-type via host `emobie-inputd` (systemd --user auto-start; off by default; see [docs/MACROS.md](docs/MACROS.md)) |
| **Summon** | Global hotkey (default `Ctrl+Shift+Space`; letters/numbers need a modifier) and system tray |
| **Layout** | Resize freely — horizontal, square, or vertical; frameless by default (optional title bar in Settings) |
| **Pin** | Always-on-top from the toolbar or tray (X11 + Plasma Wayland; other Wayland compositors may ignore) |
| **Look** | Light / dark / system theme, emoji size, and skin tone defaults |
| **Sort** | Default order, name, type, first used, or number of uses |

**Tray:** left-click shows emobie · right-click for Show / Hide / Pin / Quit  
**Close:** hides to tray — quit from Settings or the tray menu

---

## Develop

**Prerequisites**

- Node.js 20+
- Rust (stable)

Debian / Ubuntu:

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  build-essential \
  curl wget file \
  libssl-dev \
  libxdo-dev
```

Fedora:

```bash
sudo dnf install \
  webkit2gtk4.1-devel \
  libayatana-appindicator-gtk3-devel \
  librsvg2-devel \
  openssl-devel
```

**Run**

```bash
npm install
npm run tauri dev
```

```bash
npm test          # unit smoke tests
npm run build     # frontend typecheck + Vite build
```

---

## Build & package

```bash
npm run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`.

On Arch / CachyOS (and some other rolling distros), AppImage bundling needs
`NO_STRIP=true` so linuxdeploy’s outdated `strip` doesn’t choke on `.relr.dyn`
sections. If the linuxdeploy AppImage itself won’t run, also set
`APPIMAGE_EXTRACT_AND_RUN=1`.

```bash
# One format
npm run tauri build -- --bundles deb
npm run tauri build -- --bundles rpm
NO_STRIP=true APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri build -- --bundles appimage

# All three (same env for AppImage)
NO_STRIP=true APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri build -- --bundles deb,rpm,appimage
```

### Flatpak (GitHub Releases — from a `.deb`)

```bash
npm run tauri build -- --bundles deb
cp src-tauri/target/release/bundle/deb/*.deb flatpak/emobie.deb

flatpak install -y flathub org.gnome.Platform//50 org.gnome.Sdk//50
flatpak-builder --force-clean --user --repo=flatpak/repo flatpak/build-dir \
  flatpak/io.github.asafelobotomy.emobie.deb.yml
flatpak build-bundle flatpak/repo emobie.flatpak io.github.asafelobotomy.emobie
flatpak install --user emobie.flatpak
```

A separate **source-build** manifest ([`flatpak/io.github.asafelobotomy.emobie.yml`](flatpak/io.github.asafelobotomy.emobie.yml)) is prepared for a future Flathub submission. Flathub listing is **deferred** until the checklist in [`docs/FLATHUB.md`](docs/FLATHUB.md) is green.
---

## Releases

Tags matching `vX.Y.Z` run GitHub Actions to publish **`.deb`**, **`.rpm`**, **AppImage**, and **Flatpak**, with notes from [`CHANGELOG.md`](CHANGELOG.md).

```bash
# Keep package.json, Cargo.toml, and tauri.conf.json versions in sync
git tag v0.6.10
git push origin v0.6.10
```

Or run the **Release** workflow from the Actions tab.

---

## Notes

- Global shortcuts and tray icons can differ on Wayland vs X11 depending on your compositor. On **GNOME**, install an AppIndicator / KStatusNotifierItem extension for the tray ([details](docs/LINUX.md)).
- Optional **auto-paste** and **as-you-type expansion** inject keystrokes via host `emobie-inputd` (see [docs/MACROS.md](docs/MACROS.md)). Clipboard copy always works without the helper. Diagnose setup with `npm run verify:expand`.
- Flatpak preferences live under
  `~/.var/app/io.github.asafelobotomy.emobie/…`; native under
  `~/.local/share/io.github.asafelobotomy.emobie/…` (older native builds used
  `com.emobie.app`). A durable mirror at
  `~/.local/share/emobie/preferences.json` keeps favorites/macros across both.
- Pin is reliable on X11 and Plasma Wayland; other Wayland compositors may ignore it.

---

## License

[MIT](LICENSE) © aSafeLobotomy
