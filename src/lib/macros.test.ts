import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { normalizeMacros, normalizePreferences } from "./normalizePreferences.ts";
import {
  expansionMatches,
  findHotkeyConflict,
  findTriggerConflict,
  searchMacros,
  shortcodeTrigger,
  type MacroEntry,
} from "./macroHelpers.ts";
import { exportMacrosYaml, importMacrosYaml } from "./macroYaml.ts";
import type { Macro } from "../types/preferences.ts";

describe("normalizeMacros", () => {
  it("drops invalid entries and duplicate triggers", () => {
    const macros = normalizeMacros([
      { id: "1", trigger: ":a", expansion: "A", hotkey: null, enabled: true },
      { id: "2", trigger: ":a", expansion: "B", hotkey: null, enabled: true },
      { id: "", trigger: ":b", expansion: "B", hotkey: null, enabled: true },
      { id: "3", trigger: "  ", expansion: "C", hotkey: null, enabled: true },
    ]);
    assert.equal(macros.length, 1);
    assert.equal(macros[0].trigger, ":a");
  });
});

describe("normalizePreferences macros", () => {
  it("fills macro defaults", () => {
    const prefs = normalizePreferences({});
    assert.deepEqual(prefs.macros, []);
    assert.equal(prefs.showShortcodeMacros, true);
    assert.equal(prefs.autoPasteOnCopy, false);
    assert.equal(prefs.expandAsYouType, false);
    assert.equal(prefs.expandTriggerMode, "space");
  });

  it("accepts immediate expand trigger mode", () => {
    const prefs = normalizePreferences({ expandTriggerMode: "immediate" });
    assert.equal(prefs.expandTriggerMode, "immediate");
  });
});

describe("macros helpers", () => {
  it("formats shortcode triggers", () => {
    assert.equal(shortcodeTrigger("smile"), ":smile:");
    assert.equal(shortcodeTrigger(":smile:"), ":smile:");
  });

  it("searches trigger and expansion", () => {
    const list: MacroEntry[] = [
      {
        id: "1",
        trigger: ":sig",
        expansion: "Best regards",
        hotkey: null,
        enabled: true,
        source: "custom",
      },
      {
        id: "2",
        trigger: ":)",
        expansion: "🙂",
        hotkey: null,
        enabled: true,
        source: "shortcode",
      },
    ];
    assert.equal(searchMacros(list, "regards").length, 1);
    assert.equal(searchMacros(list, ":sig").length, 1);
    assert.equal(searchMacros(list, ":)").length, 1);
    assert.equal(searchMacros(list, "zzzz").length, 0);
  });

  it("applies global trigger mode to sync matches", () => {
    const list: MacroEntry[] = [
      {
        id: "1",
        trigger: ":sig",
        expansion: "Hi",
        hotkey: null,
        enabled: true,
        source: "custom",
      },
    ];
    assert.equal(expansionMatches(list, "space")[0].mode, "space");
    assert.equal(expansionMatches(list, "immediate")[0].mode, "immediate");
  });
});

describe("macro conflicts", () => {
  const macros: Macro[] = [
    {
      id: "1",
      trigger: ":a",
      expansion: "A",
      hotkey: "Control+Alt+1",
      enabled: true,
    },
  ];

  it("detects trigger clashes", () => {
    assert.ok(findTriggerConflict(macros, ":a"));
    assert.equal(findTriggerConflict(macros, ":a", "1"), null);
  });

  it("detects hotkey clashes", () => {
    assert.ok(
      findHotkeyConflict(macros, "Control+Shift+Space", "Control+Alt+1"),
    );
    assert.ok(
      findHotkeyConflict(macros, "Control+Shift+Space", "Control+Shift+Space"),
    );
    assert.equal(
      findHotkeyConflict(macros, "Control+Shift+Space", "Control+Alt+9"),
      null,
    );
  });
});

describe("macroYaml", () => {
  it("round-trips custom macros", () => {
    const macros: Macro[] = [
      {
        id: "1",
        trigger: ":sig",
        expansion: "Hello\nWorld",
        hotkey: "F9",
        enabled: true,
      },
    ];
    const yaml = exportMacrosYaml(macros);
    const result = importMacrosYaml(yaml, []);
    assert.equal(result.imported, 1);
    assert.equal(result.macros[0].trigger, ":sig");
    assert.equal(result.macros[0].expansion, "Hello\nWorld");
    assert.equal(result.macros[0].hotkey, "F9");
  });

  it("overwrites existing triggers on import", () => {
    const existing: Macro[] = [
      {
        id: "keep",
        trigger: ":sig",
        expansion: "old",
        hotkey: null,
        enabled: true,
      },
    ];
    const result = importMacrosYaml(
      `matches:\n  - trigger: ":sig"\n    replace: "new"\n`,
      existing,
    );
    assert.equal(result.macros[0].id, "keep");
    assert.equal(result.macros[0].expansion, "new");
  });
});
