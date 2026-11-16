// Node tests for the Emoji and macros page: logic.js, page.toml, and that what Page.qml sends is
// what page.toml allows and what `emobie` prints (fixtures/ are checked against the real CLI by
// src-tauri/src/cli/tests.rs).
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { loadLogic } from "./load-logic.mjs";
import { parseToml } from "./toml-lite.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const pageDir = join(here, "../../uq/pages/emobie");
const L = loadLogic(join(pageDir, "logic.js"));
const manifest = parseToml(readFileSync(join(pageDir, "page.toml"), "utf8"));
const fixture = (name) => JSON.parse(readFileSync(join(here, "fixtures", name), "utf8"));
const command = (id) => manifest.command.find((c) => c.id === id);
const pageQml = readdirSync(pageDir).filter((f) => f.endsWith(".qml")).map((f) => [f, readFileSync(join(pageDir, f), "utf8")]);

test("readStatus: reads emobie's status document", () => {
  const v = L.readStatus({ code: 0, output: fixture("status.json") });
  assert.equal(v.status, "ready");
  assert.equal(v.saved, true);
  assert.deepEqual(v.macros.map((m) => m.trigger), [".sig", ".addr"]);
  assert.deepEqual(v.deleted.map((m) => m.id), ["d1", "d2"]);
  assert.equal(v.deletedMax, 50);
  assert.equal(L.row(v, "theme").value, "dark");
  assert.equal(L.row(v, "nope").type, "", "an unknown setting reads as empty");
  assert.equal(L.readStatus({ code: 0, output: fixture("status-empty.json") }).saved, false);
});

test("readStatus: anything else is an error with a reason", () => {
  assert.equal(L.readStatus(undefined).status, "error");
  assert.match(L.readStatus({ error: "timed out" }).error, /timed out/);
  assert.equal(L.readStatus({ code: 0, output: "text" }).status, "error");
  assert.equal(L.readStatus({ code: 1, output: { ok: false, error: "no home" } }).error, "no home");
  assert.equal(L.readStatus({ code: 0, output: { config: [] } }).status, "error");
  const sparse = L.readStatus({ code: 0, output: { config: {}, macros: "x", deleted: [1, { id: "a" }] } });
  assert.deepEqual([sparse.macros, sparse.deleted.length], [[], 1]);
});

test("envelope: a refusal is shown, a cancelled confirmation is not one", () => {
  assert.deepEqual(L.envelope({ code: 0, output: { ok: true, summary: "Saved" } }), { ok: true, error: "", summary: "Saved" });
  assert.equal(L.envelope({ code: 1, error: "exit 1", output: fixture("error-theme.json") }).ok, false);
  assert.match(L.envelope({ code: 1, output: fixture("error-theme.json") }).error, /system, light, dark/);
  assert.match(L.envelope({ error: "timed out" }).error, /timed out/);
  assert.equal(L.envelope({ code: 1 }).ok, false);
  assert.equal(L.envelope(undefined).ok, false);
  assert.equal(L.isCancelled({ error: "Cancelled", cancelled: true }), true);
  assert.equal(L.isCancelled({ error: "x" }), false);
});

test("wording: one line, short hotkeys, relative dates", () => {
  assert.equal(L.oneLine("a\nb\tc", 20), "a b c");
  assert.equal(L.oneLine("x".repeat(10), 5), "xxxx…");
  assert.equal(L.oneLine(null, 5), "");
  assert.equal(L.hotkeyText("Control+Alt+S"), "Ctrl+Alt+S");
  assert.equal(L.hotkeyText("Alt+Control+S"), "Alt+Ctrl+S");
  assert.equal(L.hotkeyText(null), "");
  const now = 1_700_000_000_000;
  assert.equal(L.whenText(now - 5_000, now), "just now");
  assert.equal(L.whenText(now - 5 * 60_000, now), "5 min ago");
  assert.equal(L.whenText(now - 3_600_000, now), "1 hour ago");
  assert.equal(L.whenText(now - 3 * 3_600_000, now), "3 hours ago");
  assert.equal(L.whenText(now - 86_400_000, now), "yesterday");
  assert.equal(L.whenText(now - 5 * 86_400_000, now), "5 days ago");
  assert.equal(L.whenText(now - 90 * 86_400_000, now), "2023-08-16");
  assert.equal(L.whenText(0, now), "");
  assert.equal(L.whenText(undefined, now), "");
});

test("typed text is checked before emobie is asked, by the manifest's own rules", () => {
  const macros = [{ trigger: ".sig" }];
  assert.equal(L.triggerProblem(" .new ", macros), "");
  assert.match(L.triggerProblem(".sig", macros), /already uses/);
  assert.match(L.triggerProblem("  ", macros), /needs a trigger/);
  assert.match(L.triggerProblem("a\u0001b", macros), /control/);
  assert.match(L.triggerProblem("x".repeat(257), macros), /256/);
  assert.match(L.triggerProblem("-x", macros), /dash/);
  assert.equal(L.expansionProblem("Hello there"), "");
  assert.match(L.expansionProblem(""), /needs an expansion/);
  assert.match(L.expansionProblem("a\nb"), /emobie/);
  assert.match(L.expansionProblem("x".repeat(1001)), /1000/);
  assert.equal(L.appProblem("kate", ["keepassxc"]), "");
  assert.match(L.appProblem("KeePassXC", ["keepassxc"]), /already in the list/);
  assert.match(L.appProblem(" ", []), /name of an app/);
  assert.match(L.appProblem("-h", []), /isn't an app name/);
  assert.equal(L.idOk("0b7e2a3c-1d4f-4e5a-9b6c-7d8e9f0a1b2c"), true);
  assert.equal(L.idOk("m1"), true);
  assert.equal(L.idOk("-lead"), false);
  assert.equal(L.idOk("a b"), false);
  assert.equal(L.idOk(""), false);
});

test("manifest: what the page sends passes the argument patterns, and hostile text doesn't", () => {
  const re = (id, name) => new RegExp(command(id).args.find((a) => a.name === name).pattern, "u");
  for (const ok of ["0b7e2a3c-1d4f-4e5a-9b6c-7d8e9f0a1b2c", "m1"]) assert.ok(re("macro-remove", "id").test(ok), ok);
  for (const bad of ["", "-x", "a b", "a;b", "a\nb", "x".repeat(81), "--force"]) assert.ok(!re("macro-remove", "id").test(bad), JSON.stringify(bad));
  assert.ok(re("macro-add", "trigger").test(".sig") && re("macro-add", "trigger").test("two words"));
  assert.ok(!re("macro-add", "trigger").test("a\tb") && !re("macro-add", "trigger").test("x".repeat(257)));
  assert.ok(re("macro-add", "expansion").test("Best regards, Sam") && !re("macro-add", "expansion").test("a\nb"));
  assert.ok(!re("macro-add", "expansion").test("x".repeat(1001)));
  assert.ok(re("config-set", "value").test("Control+Alt+K") && !re("config-set", "value").test("a\u0000b"));
  assert.ok(re("excluded-add", "app").test("org.kde.kate") && !re("excluded-add", "app").test("a\u0085b"));
  for (const c of manifest.command) for (const a of c.args ?? []) assert.ok(a.pattern.startsWith("^") && a.pattern.endsWith("$"), `${c.id}.${a.name} is anchored`);
});

test("manifest: the setting keys the page may set are the ones emobie says can be set", () => {
  const config = fixture("status.json").config;
  const settable = Object.entries(config).filter(([, r]) => !r.readonly && r.type !== "apps").map(([k]) => k).sort();
  const pattern = command("config-set").args.find((a) => a.name === "key").pattern;
  assert.deepEqual(pattern.replace(/^\^\(|\)\$$/g, "").split("|").sort(), settable);
  // Every setting has words on the page, and the page has no words for one that doesn't exist.
  const withWords = Object.keys(L.META).sort();
  assert.deepEqual(withWords, settable.filter((k) => k !== "expandExcludedApps"));
  // The choices the page offers are exactly what emobie accepts.
  for (const [key, meta] of Object.entries(L.META)) {
    if (meta.choices) assert.deepEqual(meta.choices.map((c) => c[0]).sort(), [...config[key].allowed].sort(), key);
    if (meta.lowLabel !== undefined && config[key].type === "int") assert.ok(config[key].max > config[key].min, key);
  }
  assert.ok(config.autoPasteOnCopy.readonly && config.expandAsYouType.readonly, "the page only shows these two");
  for (const key of ["autoPasteOnCopy", "expandAsYouType"]) assert.ok(!(key in L.META));
});

test("manifest: every command the page runs exists, takes the arguments sent, and previews match", () => {
  const ids = new Set(manifest.command.map((c) => c.id));
  const used = new Set();
  for (const [file, src] of pageQml) {
    for (const m of src.matchAll(/(?:run|command\.run|command\.watch)\(\s*(?:[^,()]+,\s*)?"([a-z-]+)"/g)) used.add(m[1]);
    for (const line of src.split("\n").filter((l) => !/objectName/.test(l))) for (const m of line.matchAll(/"((?:config|excluded|macro)-[a-z-]+|status)"/g)) used.add(m[1]);
    assert.ok(!/\bText\s*\{|\bTextEdit\b|TextInput\b|createQmlObject|Kirigami/.test(src), `${file} shows text only through UqLabel`);
  }
  for (const id of used) assert.ok(ids.has(id), `page uses ${id}, page.toml has it`);
  for (const id of ids) assert.ok(used.has(id) || id.endsWith("-plan"), `page.toml has ${id}, the page uses it`);
  for (const c of manifest.command) {
    assert.ok(c.argv[0].startsWith("/usr/bin/"), c.id);
    if (c.preview) {
      const plan = command(c.preview);
      assert.ok(plan && !plan.confirm && !plan.preview, `${c.id} previews with a plain read`);
      assert.deepEqual((plan.args ?? []).map((a) => a.name), (c.args ?? []).map((a) => a.name), `${c.id}'s preview takes the same arguments`);
      assert.ok(plan.argv.includes("--dry-run") && plan.argv.includes("--json"), c.preview);
    }
    if (c.watch) assert.ok(!c.args && !c.confirm, `${c.id}: only a plain read can be watched`);
    if (c.id.startsWith("macro-purge") && !c.id.endsWith("-plan")) assert.ok(c.confirm && c.preview, `${c.id} asks first, with a preview`);
    assert.ok(!c.argv.includes("--force"), c.id);
  }
});

test("manifest: header, requirements and search entries", () => {
  assert.equal(manifest.uq_api, 3);
  assert.equal(manifest.id, "emobie");
  assert.deepEqual(manifest.requires.bins, ["emobie"]);
  const settingIds = new Set(manifest.setting.map((s) => s.id));
  for (const [file, src] of pageQml) {
    for (const m of src.matchAll(/settingId:\s*"([^"]+)"/g)) assert.ok(settingIds.has(m[1]), `${file}: ${m[1]} is a [[setting]]`);
  }
  for (const id of settingIds) assert.ok(pageQml.some(([, src]) => src.includes(`"${id}"`)), `${id} is on the page`);
  assert.ok(manifest.command.find((c) => c.id === "status").watch.files.length <= 8);
});

test("the page shows no emoji: it is settings, macros and deleted macros", () => {
  const text = pageQml.map(([, s]) => s).join("\n") + readFileSync(join(pageDir, "logic.js"), "utf8");
  assert.ok(!/\p{Extended_Pictographic}/u.test(text), "no emoji characters in the page");
  assert.ok(!/emoji\.json|favorites|recents/.test(text.replace(/Emoji palette|emoji grid|recently used emoji|your favourite emoji|favourite emoji|The emoji are/gi, "")), "no emoji data is read");
});
