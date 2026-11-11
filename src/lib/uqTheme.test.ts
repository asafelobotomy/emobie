import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { resolveTheme } from "./uqTheme.ts";
import {
  PROPERTIES,
  VERSION,
  watchTokens,
} from "../vendor/uq-tokens/uq-tokens.js";

describe("resolveTheme", () => {
  it("follows uQ's variant only while the setting is System", () => {
    assert.equal(resolveTheme("system", false, "dark"), "dark");
    assert.equal(resolveTheme("system", true, "light"), "light");
    assert.equal(resolveTheme("system", true, null), "dark");
    assert.equal(resolveTheme("system", false, "bogus"), "light");
    assert.equal(resolveTheme("light", true, "dark"), "light");
    assert.equal(resolveTheme("dark", false, "light"), "dark");
  });
});

function fakeRoot() {
  const props = new Map<string, string>();
  const attrs = new Map<string, string>();
  return {
    props,
    attrs,
    style: {
      setProperty: (k: string, v: string) => void props.set(k, v),
      removeProperty: (k: string) => props.delete(k),
    },
    setAttribute: (k: string, v: string) => void attrs.set(k, v),
    removeAttribute: (k: string) => void attrs.delete(k),
  };
}

describe("vendored uq-tokens, wired as useUqTokens does", () => {
  it("is the pinned version", () => {
    assert.equal(VERSION, "1.0.0");
    assert.ok(PROPERTIES.includes("--uq-bg"));
  });

  it("applies tokens, follows changes and clears when the file goes", async () => {
    const root = fakeRoot();
    let text: string | null = JSON.stringify({
      variant: "dark",
      colors: { bg: "#101010", accent: "red;}body{x:y" },
      ui: { radius: 6 },
      fonts: { ui: "Inter" },
    });
    let fire = () => {};
    let unsubscribed = false;
    const watch = watchTokens({
      read: async () => text,
      subscribe: async (cb) => {
        fire = cb;
        return () => {
          unsubscribed = true;
        };
      },
      root,
    });
    await watch.ready;
    assert.equal(root.props.get("--uq-bg"), "#101010");
    assert.equal(root.props.has("--uq-accent"), false, "injection attempt dropped");
    assert.equal(root.props.get("--uq-radius"), "6px");
    assert.equal(root.props.get("--uq-font-ui"), '"Inter"');
    assert.equal(root.attrs.get("data-uq-variant"), "dark");

    text = null;
    fire();
    await watch.refresh();
    assert.equal(root.props.size, 0);
    assert.equal(root.attrs.has("data-uq-variant"), false);

    await new Promise((r) => setTimeout(r, 0));
    watch.stop();
    assert.equal(unsubscribed, true);
  });
});
