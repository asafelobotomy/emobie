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
