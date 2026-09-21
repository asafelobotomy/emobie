#!/usr/bin/env bash
# Regenerate Flatpak offline dependency manifests from lockfiles.
# Requires: any Python 3 (cargo sources, no extra deps) and flatpak-node-generator
#   on PATH (from flatpak/flatpak-builder-tools) for the npm sources.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Generating flatpak/cargo-sources.json (app + emobie-inputd lockfiles)…"
# The Flatpak builds both crates offline, so the vendored set must cover both
# lockfiles — flatpak-cargo-generator only accepts one, hence our own script.
python3 scripts/flatpak-cargo-sources.py \
  -o flatpak/cargo-sources.json \
  src-tauri/Cargo.lock \
  crates/emobie-inputd/Cargo.lock

if ! command -v flatpak-node-generator >/dev/null 2>&1; then
  echo "flatpak-node-generator not on PATH (pip install flatpak-builder-tools/node)." >&2
  exit 1
fi

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
