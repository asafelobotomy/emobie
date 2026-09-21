#!/usr/bin/env bash
# Fail when a Tauri command is registered/invoked but not permitted by the ACL.
#
# permissions/*.toml exists, so Tauri enforces the app ACL: a command missing
# from a permission file *and* capabilities/default.json is rejected at runtime
# ("Command X not allowed by ACL") — e.g. the GNOME pin setup button once was.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export ROOT

node --input-type=module <<'EOF_NODE'
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const root = process.env.ROOT;
const read = (p) => readFileSync(join(root, p), "utf8");
const errors = [];

// 1. Commands registered in the invoke handler.
const lib = read("src-tauri/src/lib.rs");
const handler = /generate_handler!\[([\s\S]*?)\]/.exec(lib)?.[1] ?? "";
const registered = new Set(
  handler
    .split(",")
    .map((s) => s.trim().split("::").pop())
    .filter(Boolean),
);
if (registered.size === 0) errors.push("could not parse generate_handler![…] in lib.rs");

// 2. Permissions granting each command, and the capability listing them.
const permDir = join(root, "src-tauri/permissions");
const permissionFor = new Map(); // command -> identifier
for (const file of readdirSync(permDir).filter((f) => f.endsWith(".toml"))) {
  const text = readFileSync(join(permDir, file), "utf8");
  for (const block of text.split("[[permission]]").slice(1)) {
    const id = /identifier\s*=\s*"([^"]+)"/.exec(block)?.[1];
    const cmds = /commands\.allow\s*=\s*\[([^\]]*)\]/.exec(block)?.[1] ?? "";
    for (const cmd of cmds.match(/"([^"]+)"/g) ?? []) {
      permissionFor.set(cmd.replaceAll('"', ""), id);
    }
  }
}
const capability = JSON.parse(read("src-tauri/capabilities/default.json"));
const granted = new Set(capability.permissions);

for (const cmd of registered) {
  const id = permissionFor.get(cmd);
  if (!id) errors.push(`command ${cmd}: no permission in src-tauri/permissions/*.toml`);
  else if (!granted.has(id)) errors.push(`command ${cmd}: permission ${id} is not in capabilities/default.json`);
}

// 3. Everything the frontend invokes must be a registered command.
const walk = (dir) =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(join(dir, e.name)) : [join(dir, e.name)],
  );
const invoked = new Set();
for (const file of walk(join(root, "src")).filter((f) => /\.(ts|tsx)$/.test(f) && !f.endsWith(".test.ts"))) {
  const text = readFileSync(file, "utf8");
  for (const m of text.matchAll(/invoke(?:<[^>]*>)?\(\s*"([a-z0-9_]+)"/g)) invoked.add(m[1]);
}
for (const cmd of invoked) {
  if (!registered.has(cmd)) errors.push(`frontend invokes ${cmd}, which is not registered in lib.rs`);
}

if (errors.length) {
  console.error(errors.map((e) => `ACL check failed: ${e}`).join("\n"));
  process.exit(1);
}
console.log(`Tauri ACL OK (${registered.size} commands registered, ${invoked.size} invoked from the frontend)`);
EOF_NODE
