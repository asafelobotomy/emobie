#!/usr/bin/env bash
# Install emobie desktop entry + hicolor icons for a native ~/.local install.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICON_ID="io.github.asafelobotomy.emobie"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN="${XDG_BIN_HOME:-$HOME/.local/bin}/emobie"

install -Dm644 "$ROOT/flatpak/io.github.asafelobotomy.emobie.desktop" \
  "$APP_DIR/applications/${ICON_ID}.desktop"
sed -i "s|^Exec=.*|Exec=${BIN}|" "$APP_DIR/applications/${ICON_ID}.desktop"
ln -sfn "${ICON_ID}.desktop" "$APP_DIR/applications/emobie.desktop"
# Drop pre-app-id desktop entries that confuse Wayland icon matching.
rm -f "$APP_DIR/applications/com.emobie.app.desktop"

for pair in \
  "16:16x16.png" "22:22x22.png" "24:24x24.png" "32:32x32.png" \
  "48:48x48.png" "64:64x64.png" "128:128x128.png" "256:256x256.png" \
  "512:icon.png"; do
  size="${pair%%:*}"
  file="${pair##*:}"
  src="$ROOT/src-tauri/icons/$file"
  [[ -f "$src" ]] || continue
  install -Dm644 "$src" \
    "$APP_DIR/icons/hicolor/${size}x${size}/apps/${ICON_ID}.png"
  ln -sfn "${ICON_ID}.png" \
    "$APP_DIR/icons/hicolor/${size}x${size}/apps/emobie.png"
done

update-desktop-database "$APP_DIR/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "$APP_DIR/icons/hicolor" 2>/dev/null || true
echo "Installed desktop/icons for ${ICON_ID} under $APP_DIR"
