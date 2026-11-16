import assert from "node:assert/strict";
import test from "node:test";
import type { DeletedMacro, Macro } from "../types/preferences.ts";
import {
  DELETED_MACROS_MAX,
  purgeDeleted,
  restoreDeletedMacro,
  trashMacro,
} from "./deletedMacros.ts";
import { mergePreferencePartials } from "./mergePreferences.ts";
import { normalizeDeletedMacros, normalizePreferences } from "./normalizePreferences.ts";

const macro = (id: string, trigger: string, extra: Partial<Macro> = {}): Macro => ({
  id,
  trigger,
  expansion: `expansion of ${trigger}`,
  hotkey: null,
  enabled: true,
  ...extra,
});

test("deleting moves a macro to the front of the bin", () => {
  const a = macro("a", ".a");
  const b = macro("b", ".b");
  const first = trashMacro([a, b], [], "a", 10);
  assert.deepEqual(first.macros, [b]);
  assert.deepEqual(first.deletedMacros, [{ ...a, deletedAt: 10 }]);
  const second = trashMacro(first.macros, first.deletedMacros, "b", 20);
  assert.deepEqual(second.deletedMacros.map((m) => m.id), ["b", "a"]);
  assert.equal(trashMacro([a], [], "missing", 1).macros.length, 1);
});

test("the bin keeps the newest fifty", () => {
  let state = { macros: [] as Macro[], deletedMacros: [] as DeletedMacro[] };
  for (let n = 0; n < DELETED_MACROS_MAX + 3; n++) {
    state = trashMacro([macro(`m${n}`, `.m${n}`)], state.deletedMacros, `m${n}`, n + 1);
  }
  assert.equal(state.deletedMacros.length, DELETED_MACROS_MAX);
  assert.equal(state.deletedMacros[0].id, `m${DELETED_MACROS_MAX + 2}`);
});

test("restoring puts a macro back without its deletion time", () => {
  const a = macro("a", ".a");
  const result = restoreDeletedMacro([], [{ ...a, deletedAt: 5 }], "a", "Control+Shift+Space");
  assert.ok(!("error" in result));
  assert.deepEqual(result.macros, [a]);
  assert.deepEqual(result.deletedMacros, []);
});

test("restoring is refused when the trigger was taken, and drops a taken hotkey", () => {
  const gone: DeletedMacro = { ...macro("a", ".a"), deletedAt: 5 };
  const clash = restoreDeletedMacro([macro("x", ".a")], [gone], "a", "Control+Shift+Space");
  assert.ok("error" in clash);

  const withKey: DeletedMacro = { ...macro("b", ".b", { hotkey: "Control+Alt+B" }), deletedAt: 6 };
  const taken = restoreDeletedMacro(
    [macro("y", ".y", { hotkey: "Control+Alt+B" })],
    [withKey],
    "b",
    "Control+Shift+Space",
  );
  assert.ok(!("error" in taken));
  assert.equal(taken.macros.find((m) => m.id === "b")?.hotkey, null);
  assert.ok("error" in restoreDeletedMacro([], [], "zzz", "x"));
});

test("purging removes one or all", () => {
  const bin: DeletedMacro[] = [
    { ...macro("a", ".a"), deletedAt: 2 },
    { ...macro("b", ".b"), deletedAt: 1 },
  ];
  assert.deepEqual(purgeDeleted(bin, "a").map((m) => m.id), ["b"]);
  assert.deepEqual(purgeDeleted(bin, null), []);
});

test("saved deleted macros are cleaned like the app cleans macros", () => {
  const out = normalizeDeletedMacros([
    { ...macro("a", ".a"), deletedAt: 1 },
    { ...macro("b", ".a"), deletedAt: 9 },
    { ...macro("a", ".dup"), deletedAt: 3 },
    { id: "c", trigger: "", expansion: "x", deletedAt: 4 },
    "junk",
  ]);
  assert.deepEqual(out.map((m) => m.id), ["b", "a"], "newest first; a shared trigger is fine, a shared id is not");
  assert.deepEqual(normalizeDeletedMacros(undefined), []);
  assert.deepEqual(normalizePreferences({}).deletedMacros, []);
});

test("an empty bin in the primary store is not refilled from a snapshot", () => {
  const stale = { deletedMacros: [{ ...macro("a", ".a"), deletedAt: 1 }] };
  assert.deepEqual(mergePreferencePartials({ deletedMacros: [] }, [stale]).deletedMacros, []);
  assert.equal(mergePreferencePartials({}, [stale]).deletedMacros?.length, 1, "recovered on a fresh store");
});

test("preferences compare equal whatever their key order", async () => {
  const { samePreferences } = await import("./samePreferences.ts");
  assert.ok(samePreferences({ a: 1, b: { c: [1, 2], d: 3 } }, { b: { d: 3, c: [1, 2] }, a: 1 }));
  assert.ok(!samePreferences({ a: 1 }, { a: 2 }));
  assert.ok(!samePreferences({ a: [1, 2] }, { a: [2, 1] }));
  assert.ok(samePreferences({ a: 1, x: undefined }, { a: 1 }));
});
