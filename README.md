<p align="center">
  <img src="branding/emobie-icon-transparent.png" alt="emobie" width="128" height="128" />
</p>

<h1 align="center">emobie</h1>

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
| **Flatpak** | Sandboxed desktop install | `flatpak install --user emobie-*-x86_64.flatpak` |
| **`.deb`** | Debian / Ubuntu / Mint | `sudo apt install ./emobie_*_amd64.deb` |
| **`.rpm`** | Fedora / RHEL / openSUSE | `sudo dnf install ./emobie-*-1.x86_64.rpm` |
| **AppImage** | Portable, no install | `chmod +x emobie_*.AppImage && ./emobie_*.AppImage` |

```bash
# Flatpak (after downloading the release bundle)
flatpak install --user emobie-*-x86_64.flatpak
flatpak run io.github.asafelobotomy.Emobie
```

App ID: `io.github.asafelobotomy.Emobie`

---

## Features

| | |
|---|---|
| **Catalog** | Full Unicode emoji set via emojibase, with categories and search (name, tags, shortcodes) |
| **Favorites & recents** | Right-click to favorite; recent history with configurable size |
| **Copy** | One-click clipboard copy (no auto-paste into other apps) |
| **Summon** | Global hotkey (default `Ctrl+Shift+Space`; letters/numbers need a modifier) and system tray |
| **Layout** | Resize freely — horizontal, square, or vertical; frameless by default (optional title bar in Settings) |
| **Pin** | Always-on-top from the toolbar or tray |
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

```bash
# One format
npm run tauri build -- --bundles deb
npm run tauri build -- --bundles rpm
npm run tauri build -- --bundles appimage

# All three
npm run tauri build -- --bundles deb,rpm,appimage
```

### Flatpak (from a `.deb`)

```bash
npm run tauri build -- --bundles deb
cp src-tauri/target/release/bundle/deb/*.deb flatpak/emobie.deb

flatpak install -y flathub org.gnome.Platform//50 org.gnome.Sdk//50
flatpak-builder --force-clean --user --repo=flatpak/repo flatpak/build-dir \
  flatpak/io.github.asafelobotomy.Emobie.yml
flatpak build-bundle flatpak/repo emobie.flatpak io.github.asafelobotomy.Emobie
flatpak install --user emobie.flatpak
```

---

## Releases

Tags matching `vX.Y.Z` run GitHub Actions to publish **`.deb`**, **`.rpm`**, **AppImage**, and **Flatpak**, with notes from [`CHANGELOG.md`](CHANGELOG.md).

```bash
# Keep package.json, Cargo.toml, and tauri.conf.json versions in sync
git tag v0.6.4
git push origin v0.6.4
```

Or run the **Release** workflow from the Actions tab.

---

## Notes

- Global shortcuts and tray icons can differ on Wayland vs X11 depending on your compositor.
- emobie copies to the clipboard only — it does not inject into the focused app.
- Flatpak preferences live under  
  `~/.var/app/io.github.asafelobotomy.Emobie/data/com.emobie.app/`.

---

## License

[MIT](LICENSE) © aSafeLobotomy
