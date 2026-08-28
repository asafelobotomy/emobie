import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { mergePreferencePartials } from "./mergePreferences.ts";
import type { Macro, Preferences } from "../types/preferences.ts";

const macro = (id: string, trigger: string): Macro => ({
  id,
  trigger,
  expansion: `${id}-out`,
  hotkey: null,
  enabled: true,
});

describe("mergePreferencePartials", () => {
  it("unions macros favorites and recents across sources", () => {
    const primary: Partial<Preferences> = {
      theme: "dark",
      macros: [macro("a", ":a")],
      favorites: ["1F600"],
      recents: ["😀"],
    };
    const other: Partial<Preferences> = {
      theme: "light",
      macros: [macro("b", ":b")],
      favorites: ["1F602", "1F600"],
      recents: ["😂", "😀"],
    };

    const merged = mergePreferencePartials(primary, [other]);
    assert.equal(merged.theme, "dark");
    assert.deepEqual(
      (merged.macros ?? []).map((item) => item.trigger).sort(),
      [":a", ":b"],
    );
    assert.deepEqual(merged.favorites, ["1F600", "1F602"]);
    assert.deepEqual(merged.recents, ["😀", "😂"]);
  });

  it("merges usage maps with max counts and earliest first-used", () => {
    const merged = mergePreferencePartials(
      { usageCounts: { a: 2 }, firstUsedAt: { a: 100 } },
      [{ usageCounts: { a: 5, b: 1 }, firstUsedAt: { a: 50, b: 90 } }],
    );
    assert.deepEqual(merged.usageCounts, { a: 5, b: 1 });
    assert.deepEqual(merged.firstUsedAt, { a: 50, b: 90 });
  });

  it("recovers user data when primary store is empty", () => {
    const merged = mergePreferencePartials(
      {},
      [
        {
          macros: [macro("sig", ":sig")],
          favorites: ["1F44D"],
          recents: ["👍"],
        },
      ],
    );
    assert.equal(merged.macros?.[0]?.trigger, ":sig");
    assert.deepEqual(merged.favorites, ["1F44D"]);
    assert.deepEqual(merged.recents, ["👍"]);
  });
});
