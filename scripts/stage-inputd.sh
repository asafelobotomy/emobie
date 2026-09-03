#!/usr/bin/env bash
# Build emobie-inputd and stage files for deb/rpm/AppImage/Flatpak host bootstrap.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="$ROOT/src-tauri/inputd-bundle"
HOST_STAGE="$STAGE/host-bundle"
mkdir -p "$STAGE" "$HOST_STAGE"

cargo build --release --manifest-path "$ROOT/crates/emobie-inputd/Cargo.toml"
install -m 755 "$ROOT/crates/emobie-inputd/target/release/emobie-inputd" "$STAGE/emobie-inputd"
install -m 644 "$ROOT/packaging/systemd/emobie-inputd.service" "$STAGE/emobie-inputd.service"
install -m 755 "$ROOT/packaging/setup-input-access.sh" "$STAGE/setup-input-access.sh"
install -m 644 "$ROOT/packaging/udev/99-emobie-input.rules" "$STAGE/99-emobie-input.rules"
install -m 644 "$ROOT/packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy" \
  "$STAGE/io.github.asafelobotomy.emobie.inputd.policy"
install -m 755 "$ROOT/packaging/bootstrap-inputd-host.sh" "$STAGE/bootstrap-inputd-host.sh"
mkdir -p "$STAGE/selinux"
install -m 644 "$ROOT/packaging/selinux/emobie-inputd.te" "$STAGE/selinux/emobie-inputd.te"

# Keep in sync with src-tauri/src/input_helper/bootstrap.rs TAR_MEMBERS.
HOST_MEMBERS=(
  emobie-inputd
  bootstrap-inputd-host.sh
  setup-input-access.sh
  99-emobie-input.rules
  io.github.asafelobotomy.emobie.inputd.policy
  selinux/emobie-inputd.te
)

rm -rf "$HOST_STAGE"
mkdir -p "$HOST_STAGE/selinux"
for member in "${HOST_MEMBERS[@]}"; do
  install -D -m "$(stat -c '%a' "$STAGE/$member")" "$STAGE/$member" "$HOST_STAGE/$member"
done
# Explicit member list (not ".") so tar -tzf lists paths without a "./" prefix —
# release CI matches exact names.
tar czf "$STAGE/inputd-host-bundle.tgz" -C "$HOST_STAGE" "${HOST_MEMBERS[@]}"

echo "Staged input helper at $STAGE (host bundle: inputd-host-bundle.tgz)"
