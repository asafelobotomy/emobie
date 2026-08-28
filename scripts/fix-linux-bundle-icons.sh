#!/usr/bin/env bash
# Post-process Tauri Linux bundles so icons/desktop match the GTK app id
# (io.github.asafelobotomy.emobie). Tauri installs them as "emobie" by default.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUNDLE="${1:-$ROOT/src-tauri/target/release/bundle}"
ICON_ID="io.github.asafelobotomy.emobie"

fix_hicolor() {
  local icons_root="$1"
  [[ -d "$icons_root" ]] || return 0

  # Tauri maps 128x128@2x → invalid hicolor/256x256@2 — normalize to 256x256.
  if [[ -d "$icons_root/256x256@2/apps" ]]; then
    mkdir -p "$icons_root/256x256/apps"
    for f in "$icons_root/256x256@2/apps/"*; do
      [[ -e "$f" ]] || continue
      local base ext
      base="$(basename "$f")"
      ext="${base##*.}"
      mv -f "$f" "$icons_root/256x256/apps/${ICON_ID}.${ext}"
    done
    rm -rf "$icons_root/256x256@2"
  fi

  for apps in "$icons_root"/*/apps; do
    [[ -d "$apps" ]] || continue
    if [[ -f "$apps/emobie.png" ]]; then
      mv -f "$apps/emobie.png" "$apps/${ICON_ID}.png"
    fi
    if [[ -f "$apps/emobie.svg" ]]; then
      mv -f "$apps/emobie.svg" "$apps/${ICON_ID}.svg"
    fi
  done
}

fix_desktop_dir() {
  local apps="$1"
  [[ -d "$apps" ]] || return 0
  local src=""
  if [[ -f "$apps/emobie.desktop" ]]; then
    src="$apps/emobie.desktop"
  elif [[ -f "$apps/${ICON_ID}.desktop" ]]; then
    src="$apps/${ICON_ID}.desktop"
  else
    return 0
  fi
  sed -i \
    -e "s|^Icon=.*|Icon=${ICON_ID}|" \
    -e "s|^StartupWMClass=.*|StartupWMClass=${ICON_ID}|" \
    "$src"
  if [[ "$(basename "$src")" != "${ICON_ID}.desktop" ]]; then
    mv -f "$src" "$apps/${ICON_ID}.desktop"
  fi
  # Convenience alias for binary-name lookups.
  ln -sfn "${ICON_ID}.desktop" "$apps/emobie.desktop"
}

fix_appdir() {
  local appdir="$1"
  [[ -d "$appdir" ]] || return 0
  echo "Fixing AppDir icons: $appdir"
  fix_hicolor "$appdir/usr/share/icons/hicolor"
  fix_desktop_dir "$appdir/usr/share/applications"
  # AppImage root icon should be a decent size, not 32px.
  if [[ -f "$appdir/usr/share/icons/hicolor/256x256/apps/${ICON_ID}.png" ]]; then
    cp -f "$appdir/usr/share/icons/hicolor/256x256/apps/${ICON_ID}.png" "$appdir/${ICON_ID}.png"
    ln -sfn "${ICON_ID}.png" "$appdir/emobie.png"
  elif [[ -f "$appdir/usr/share/icons/hicolor/128x128/apps/${ICON_ID}.png" ]]; then
    cp -f "$appdir/usr/share/icons/hicolor/128x128/apps/${ICON_ID}.png" "$appdir/${ICON_ID}.png"
    ln -sfn "${ICON_ID}.png" "$appdir/emobie.png"
  fi
  if [[ -f "$appdir/usr/share/applications/${ICON_ID}.desktop" ]]; then
    ln -sfn "usr/share/applications/${ICON_ID}.desktop" "$appdir/${ICON_ID}.desktop"
    ln -sfn "${ICON_ID}.desktop" "$appdir/emobie.desktop"
  fi
}

repack_deb() {
  local deb="$1"
  local tmp
  tmp="$(mktemp -d)"
  (
    cd "$tmp"
    ar x "$deb"
    mkdir data control
    tar -C data -xf data.tar.*
    tar -C control -xf control.tar.*
    fix_hicolor "$tmp/data/usr/share/icons/hicolor"
    fix_desktop_dir "$tmp/data/usr/share/applications"
    # Ensure expected sizes from repo icons if bundler omitted them.
    local share="$tmp/data/usr/share/icons/hicolor"
    local src_icons="$ROOT/src-tauri/icons"
    for pair in \
      "16:16x16.png" "22:22x22.png" "24:24x24.png" "32:32x32.png" \
      "48:48x48.png" "64:64x64.png" "128:128x128.png" "256:256x256.png" \
      "512:icon.png"; do
      local size="${pair%%:*}"
      local file="${pair##*:}"
      [[ -f "$src_icons/$file" ]] || continue
      install -Dm644 "$src_icons/$file" \
        "$share/${size}x${size}/apps/${ICON_ID}.png"
    done
    if [[ -f "$tmp/data/usr/share/applications/${ICON_ID}.desktop" ]]; then
      : # ok
    elif [[ -f "$ROOT/flatpak/io.github.asafelobotomy.emobie.desktop" ]]; then
      install -Dm644 "$ROOT/flatpak/io.github.asafelobotomy.emobie.desktop" \
        "$tmp/data/usr/share/applications/${ICON_ID}.desktop"
      ln -sfn "${ICON_ID}.desktop" \
        "$tmp/data/usr/share/applications/emobie.desktop"
    fi
    tar -C data -cJf data.tar.xz .
    tar -C control -czf control.tar.gz .
    local out="$tmp/fixed.deb"
    ar rcs "$out" debian-binary control.tar.gz data.tar.xz
    cp -f "$out" "$deb"
  )
  rm -rf "$tmp"
  echo "Repacked deb icons: $deb"
}

repack_rpm() {
  # RPM rewrite is heavier; fix the extracted rpmbuild tree if present, else skip.
  local rpm_dir="$BUNDLE/rpm"
  [[ -d "$rpm_dir" ]] || return 0
  find "$rpm_dir" -type d -name hicolor 2>/dev/null | while read -r h; do
    fix_hicolor "$h"
  done
  find "$rpm_dir" -type d -name applications 2>/dev/null | while read -r a; do
    fix_desktop_dir "$a"
  done
}

repack_appimage() {
  local appimage="$1"
  command -v appimagetool >/dev/null 2>&1 || {
    echo "appimagetool not found; fixing AppDir only (re-run linuxdeploy/appimagetool to refresh .AppImage)" >&2
    return 0
  }
  local appdir
  appdir="$(find "$BUNDLE/appimage" -maxdepth 1 -type d -name '*.AppDir' | head -n1 || true)"
  if [[ -z "$appdir" ]]; then
    echo "No AppDir found under $BUNDLE/appimage" >&2
    return 0
  fi
  fix_appdir "$appdir"
  ARCH=x86_64 appimagetool "$appdir" "$appimage"
  chmod +x "$appimage"
  echo "Repacked AppImage icons: $appimage"
}

main() {
  echo "Fixing Linux bundle icons under $BUNDLE"
  local deb appimage
  deb="$(find "$BUNDLE/deb" -name '*.deb' 2>/dev/null | head -n1 || true)"
  if [[ -n "$deb" ]]; then
    repack_deb "$deb"
  fi
  # Unpackaged deb data dir (tauri intermediate)
  if [[ -d "$BUNDLE/deb" ]]; then
    find "$BUNDLE/deb" -type d -path '*/data/usr/share/icons/hicolor' 2>/dev/null | while read -r h; do
      fix_hicolor "$h"
    done
    find "$BUNDLE/deb" -type d -path '*/data/usr/share/applications' 2>/dev/null | while read -r a; do
      fix_desktop_dir "$a"
    done
  fi
  repack_rpm
  local appdir
  appdir="$(find "$BUNDLE/appimage" -maxdepth 1 -type d -name '*.AppDir' 2>/dev/null | head -n1 || true)"
  if [[ -n "$appdir" ]]; then
    fix_appdir "$appdir"
  fi
  appimage="$(find "$BUNDLE/appimage" -name '*.AppImage' 2>/dev/null | head -n1 || true)"
  if [[ -n "$appimage" && -n "$appdir" ]]; then
    # Prefer refreshing AppImage when appimagetool exists; otherwise AppDir is fixed for next pack.
    if command -v appimagetool >/dev/null 2>&1; then
      ARCH=x86_64 appimagetool "$appdir" "$appimage"
      chmod +x "$appimage"
      echo "Repacked AppImage icons: $appimage"
    fi
  fi
  echo "Done."
}

main "$@"
