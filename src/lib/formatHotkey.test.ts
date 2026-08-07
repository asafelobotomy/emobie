import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { formatHotkey } from "./formatHotkey.ts";
import { SORT_OPTIONS } from "../types/preferences.ts";

describe("formatHotkey", () => {
  it("rejects bare letter keys", () => {
    assert.equal(
      formatHotkey({
        key: "a",
        ctrlKey: false,
        shiftKey: false,
        altKey: false,
        metaKey: false,
      }),
      null,
    );
  });

  it("rejects Shift+letter without Ctrl/Alt/Meta", () => {
    assert.equal(
      formatHotkey({
        key: "a",
        ctrlKey: false,
        shiftKey: true,
        altKey: false,
        metaKey: false,
      }),
      null,
    );
  });

  it("accepts Control+letter", () => {
    assert.equal(
      formatHotkey({
        key: "a",
        ctrlKey: true,
        shiftKey: false,
        altKey: false,
        metaKey: false,
      }),
      "Control+A",
    );
  });

  it("accepts bare F-keys", () => {
    assert.equal(
      formatHotkey({
        key: "F2",
        ctrlKey: false,
        shiftKey: false,
        altKey: false,
        metaKey: false,
      }),
      "F2",
    );
  });
});

describe("SORT_OPTIONS", () => {
  it("labels dateAdded as First used", () => {
    const option = SORT_OPTIONS.find((item) => item.value === "dateAdded");
    assert.ok(option);
    assert.equal(option.label, "First used");
  });
});
