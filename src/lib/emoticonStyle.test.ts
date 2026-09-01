import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  filterEmoticonsByStyle,
  toClassicVariant,
  toMinimalVariant,
} from "./emoticonStyle.ts";

describe("emoticonStyle", () => {
  it("converts between minimal and classic variants", () => {
    assert.equal(toMinimalVariant(":-)"), ":)");
    assert.equal(toClassicVariant(":)"), ":-)");
    assert.equal(toMinimalVariant(";-)"), ";)");
    assert.equal(toClassicVariant(";)"), ";-)");
  });

  it("prefers minimal forms when both exist", () => {
    const filtered = filterEmoticonsByStyle([":)", ":-)", "=)"], "minimal");
    assert.deepEqual(filtered, [":)", "=)"]);
  });

  it("prefers classic forms when both exist", () => {
    const filtered = filterEmoticonsByStyle([":)", ":-)", "=)"], "classic");
    assert.deepEqual(filtered, [":-)", "=)"]);
  });

  it("keeps lone variants and neutral emoticons", () => {
    assert.deepEqual(filterEmoticonsByStyle(["<3", "T_T"], "minimal"), [
      "<3",
      "T_T",
    ]);
    assert.deepEqual(filterEmoticonsByStyle([":-)"], "minimal"), [":-)"]);
    assert.deepEqual(filterEmoticonsByStyle([":)"], "classic"), [":)"]);
  });
});
