#!/usr/bin/env bash
# Build emobie-inputd and stage files for the Linux deb/rpm bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="$ROOT/src-tauri/inputd-bundle"
mkdir -p "$STAGE"

cargo build --release --manifest-path "$ROOT/crates/emobie-inputd/Cargo.toml"
install -m 755 "$ROOT/crates/emobie-inputd/target/release/emobie-inputd" "$STAGE/emobie-inputd"
install -m 644 "$ROOT/packaging/systemd/emobie-inputd.service" "$STAGE/emobie-inputd.service"
install -m 755 "$ROOT/packaging/setup-input-access.sh" "$STAGE/setup-input-access.sh"
install -m 644 "$ROOT/packaging/udev/99-emobie-input.rules" "$STAGE/99-emobie-input.rules"
install -m 644 "$ROOT/packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy" \
  "$STAGE/io.github.asafelobotomy.emobie.inputd.policy"

echo "Staged input helper at $STAGE"
