# Emobie

Compact Linux-first emoji palette for content creators. Browse the full Unicode emoji set, search by name or keyword, one-click copy, pin above other windows, and summon Emobie from a global hotkey or the system tray.

## Features

- Complete emoji catalog (emojibase), organized by category
- Favorites section (right-click to favorite / unfavorite)
- Search by label, tags, and shortcodes
- One-click copy to clipboard with recent history
- Always-on-top pin
- Light / dark / system theme
- Freely resizable compact window (horizontal, square, or vertical layout)
- Global hotkey (default `Ctrl+Shift+Space`) to show/hide
- System tray: left-click to show, menu for Show/Hide, Pin, Quit
- Close button hides to tray instead of quitting
- Preferences for theme, emoji size, recent max, skin tone, and hotkey

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

## Notes

- Global shortcuts and tray icons can behave differently on Wayland vs X11 depending on your compositor.
- Emobie copies to the clipboard only (it does not auto-paste into the focused app).
- Quit from the tray menu — closing the window keeps Emobie running in the tray.
