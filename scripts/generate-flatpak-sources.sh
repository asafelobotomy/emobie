#!/usr/bin/env bash
# Regenerate Flatpak offline dependency manifests from lockfiles.
# Requires: Python env with flatpak-node-generator + cargo generator deps
#   (aiohttp, tomlkit, pyyaml) and a checkout of flatpak/flatpak-builder-tools.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TOOLS_DIR="${FLATPAK_BUILDER_TOOLS:-}"
if [[ -z "$TOOLS_DIR" ]]; then
  if [[ -d /tmp/flatpak-builder-tools ]]; then
    TOOLS_DIR=/tmp/flatpak-builder-tools
  elif [[ -d "$HOME/src/flatpak-builder-tools" ]]; then
    TOOLS_DIR="$HOME/src/flatpak-builder-tools"
  else
    echo "Set FLATPAK_BUILDER_TOOLS to a flatpak-builder-tools checkout." >&2
    exit 1
  fi
fi

if ! command -v flatpak-node-generator >/dev/null 2>&1; then
  echo "flatpak-node-generator not on PATH (pip install flatpak-builder-tools/node)." >&2
  exit 1
fi

echo "Generating flatpak/cargo-sources.json…"
python3 "$TOOLS_DIR/cargo/flatpak-cargo-generator.py" \
  src-tauri/Cargo.lock \
  -o flatpak/cargo-sources.json

echo "Generating flatpak/node-sources.json…"
MOVED_NODE_MODULES=0
if [[ -d node_modules ]]; then
  mv node_modules /tmp/emobie-node_modules.flatpak-gen.$$
  MOVED_NODE_MODULES=1
fi
trap 'if [[ "$MOVED_NODE_MODULES" -eq 1 && -d /tmp/emobie-node_modules.flatpak-gen.$$ ]]; then mv /tmp/emobie-node_modules.flatpak-gen.$$ node_modules; fi' EXIT

flatpak-node-generator npm package-lock.json -o flatpak/node-sources.json

echo "Done."
echo "Remember to bump the git tag/commit in flatpak/io.github.asafelobotomy.emobie.yml when packaging a new release."
