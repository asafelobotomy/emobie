import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { errorMessage } from "./errorMessage.ts";

describe("errorMessage", () => {
  it("returns Tauri string rejections verbatim", () => {
    assert.equal(errorMessage("Command x not allowed by ACL", "fallback"), "Command x not allowed by ACL");
  });
  it("returns Error messages", () => {
    assert.equal(errorMessage(new Error("boom"), "fallback"), "boom");
  });
  it("falls back for empty or unknown values", () => {
    assert.equal(errorMessage("  ", "fallback"), "fallback");
    assert.equal(errorMessage(undefined, "fallback"), "fallback");
    assert.equal(errorMessage({ code: 1 }, "fallback"), "fallback");
  });
});
