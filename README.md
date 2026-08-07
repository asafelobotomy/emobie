# emobie

Compact Linux-first emoji palette for content creators. Browse the full Unicode emoji set, search by name or keyword, one-click copy, pin above other windows, and summon Emobie from a global hotkey or the system tray.

## Features

- Complete emoji catalog (emojibase), organized by category
- Favorites section (right-click to favorite / unfavorite)
- Search by label, tags, and shortcodes
- One-click copy to clipboard with recent history
- Always-on-top pin
- Light / dark / system theme
- Freely resizable compact window (horizontal, square, or vertical layout)
- Global hotkey (default `Ctrl+Shift+Space`) to show/hide — letters/numbers need a modifier
- System tray: left-click shows Emobie; right-click opens Show/Hide/Pin/Quit
- Close button hides to tray instead of quitting
- Preferences for theme, emoji size, recent max, skin tone, hotkey, and title bar
- Title bar hidden by default (drag from the toolbar); optional in Settings
- Flatpak packaging (`io.github.asafelobotomy.Emobie`)
- Also ships as `.deb`, `.rpm`, and AppImage from GitHub Releases

## Prerequisites (Linux)

- Node.js 20+
- Rust (stable)
- System packages (Debian/Ubuntu-style):

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libxdo-dev
```

On Fedora:

```bash
sudo dnf install webkit2gtk4.1-devel libayatana-appindicator-gtk3-devel librsvg2-devel openssl-devel
```

## Develop

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

The packaged app lands under `src-tauri/target/release/bundle/`.

Linux package formats:

```bash
# Debian / Ubuntu
npm run tauri build -- --bundles deb

# Fedora / RHEL / openSUSE
npm run tauri build -- --bundles rpm

# Portable AppImage
npm run tauri build -- --bundles appimage

# All three at once
npm run tauri build -- --bundles deb,rpm,appimage
```

### Flatpak (from a built `.deb`)

```bash
# Build the Debian package first
npm run tauri build -- --bundles deb

# Stage it for the Flatpak manifest
cp src-tauri/target/release/bundle/deb/*.deb flatpak/emobie.deb

# Build and install a local Flatpak
flatpak install -y flathub org.gnome.Platform//50 org.gnome.Sdk//50
flatpak-builder --force-clean --user --repo=flatpak/repo flatpak/build-dir \
  flatpak/io.github.asafelobotomy.Emobie.yml
flatpak build-bundle flatpak/repo emobie.flatpak io.github.asafelobotomy.Emobie
flatpak install --user emobie.flatpak
flatpak run io.github.asafelobotomy.Emobie
```

## Releases

Tagged versions (`vX.Y.Z`) trigger GitHub Actions to:

1. Build Linux `.deb`, `.rpm`, and AppImage
2. Wrap the `.deb` as a `.flatpak` bundle
3. Publish a GitHub Release with all artifacts and notes from [`CHANGELOG.md`](CHANGELOG.md)

Create a release:

```bash
# Ensure package.json, Cargo.toml, and tauri.conf.json versions match
git tag v0.6.4
git push origin v0.6.4
```

Or run the **Release** workflow manually from the Actions tab.

## Notes

- Global shortcuts and tray icons can behave differently on Wayland vs X11 depending on your compositor.
- Emobie copies to the clipboard only (it does not auto-paste into the focused app).
- Quit from the tray menu — closing the window keeps Emobie running in the tray.
- Flatpak app data (preferences store, etc.) lives under `~/.var/app/io.github.asafelobotomy.Emobie/data/com.emobie.app/`.
- See [`CHANGELOG.md`](CHANGELOG.md) for version history.
