import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { summonHotkeyEnabled } from "./uqDesktop.ts";

describe("summonHotkeyEnabled", () => {
  const base = { ready: true, allowMultipleInstances: false, uqDesktop: false };

  it("is on outside uQ once ready", () => {
    assert.equal(summonHotkeyEnabled(base), true);
    assert.equal(summonHotkeyEnabled({ ...base, ready: false }), false);
    assert.equal(summonHotkeyEnabled({ ...base, allowMultipleInstances: true }), false);
  });

  it("stays off under uQ and while the check is pending", () => {
    assert.equal(summonHotkeyEnabled({ ...base, uqDesktop: true }), false);
    assert.equal(summonHotkeyEnabled({ ...base, uqDesktop: null }), false);
  });
});
