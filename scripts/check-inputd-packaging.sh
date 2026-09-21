#!/usr/bin/env bash
# Ensure deb/rpm bundle maps include the staged inputd assets.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CONF="$ROOT/src-tauri/tauri.conf.json"

node --input-type=module <<'EOF'
import { readFileSync } from "node:fs";

const conf = JSON.parse(readFileSync(process.env.CONF, "utf8"));
const need = [
  "/usr/bin/emobie-inputd",
  "/usr/lib/systemd/user/emobie-inputd.service",
  "/usr/share/emobie/setup-input-access.sh",
  "/usr/share/emobie/bootstrap-inputd-host.sh",
  "/usr/share/emobie/99-emobie-input.rules",
  "/usr/share/emobie/selinux/emobie-inputd.te",
  "/usr/share/polkit-1/actions/io.github.asafelobotomy.emobie.inputd.policy",
];

for (const bundle of ["deb", "rpm"]) {
  const files = conf?.bundle?.linux?.[bundle]?.files;
  if (!files || typeof files !== "object") {
    console.error(`Missing bundle.linux.${bundle}.files`);
    process.exit(1);
  }
  for (const path of need) {
    if (!(path in files)) {
      console.error(`Missing ${path} under linux.${bundle}.files`);
      process.exit(1);
    }
    if (!String(files[path]).includes("inputd-bundle/")) {
      console.error(`${path} in ${bundle} must point under inputd-bundle/`);
      process.exit(1);
    }
  }
}
console.log("inputd packaging maps OK (deb + rpm)");
EOF

# Every copy of the hardened unit must tolerate a missing state directory,
# otherwise systemd fails the unit with 226/NAMESPACE on a fresh account.
for f in \
  "$ROOT/packaging/systemd/emobie-inputd.service" \
  "$ROOT/packaging/bootstrap-inputd-host.sh" \
  "$ROOT/packaging/install-inputd-user.sh" \
  "$ROOT/src-tauri/src/updates/native.rs"; do
  if grep -q 'ReadWritePaths=%h' "$f"; then
    echo "$f: ReadWritePaths must be prefixed with '-' (missing dir => 226/NAMESPACE)" >&2
    exit 1
  fi
done

# The shipped udev rule must not grant keyboard *read* access (event nodes).
if grep -Ev '^[[:space:]]*(#|$)' "$ROOT/packaging/udev/99-emobie-input.rules" | grep -q 'event\*'; then
  echo "packaging/udev/99-emobie-input.rules grants keyboard event access" >&2
  exit 1
fi

# The host bundle must be built with explicit members, never ".", so member
# names carry no "./" prefix (the app extracts by exact name).
if grep -qE 'inputd-host-bundle\.tgz" -C "\$HOST" \.$' "$ROOT"/flatpak/*.yml; then
  echo "a flatpak manifest builds inputd-host-bundle.tgz from '.' (members get a ./ prefix)" >&2
  exit 1
fi
echo "inputd unit/udev/bundle checks OK"

