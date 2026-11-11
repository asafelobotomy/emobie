import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  EMOJI_DATA_FORMAT,
  buildEmojiData,
  serializeEmojiData,
  type BuildInput,
} from "./buildEmojiData.ts";

const input: BuildInput = {
  source: "fixture 1.0.0",
  messages: {
    groups: [
      { key: "smileys-emotion", message: "smileys & emotion", order: 0 },
      { key: "component", message: "component", order: 2 },
    ],
  },
  data: [
    { emoji: "😃", label: "grinning face with big eyes", hexcode: "1F603", type: 1, group: 0, order: 2, tags: ["happy"] },
    { emoji: "😀", label: "grinning face", hexcode: "1F600", type: 1, group: 0, order: 1, emoticon: ":D" },
    { emoji: "🏻", label: "light skin tone", hexcode: "1F3FB", type: 1, group: 2, order: 9 },
    { emoji: "🇦", label: "regional indicator A", hexcode: "1F1E6", type: 1 },
    { emoji: "☺", label: "text style", hexcode: "263A", type: 0, group: 0, order: 3 },
    {
      emoji: "👋",
      label: "waving hand",
      hexcode: "1F44B",
      type: 1,
      group: 0,
      order: 4,
      skins: [{ emoji: "👋🏻", tone: 1 }, { emoji: "👋🏼", tone: [2, 3] }],
    },
  ],
  shortcodePacks: [{ "1F600": ["grinning", "Grin"] }, { "1F600": "grin", "1F603": "smiley" }],
  extraEmoticonsFor: (e) => (e === "😀" ? [":D", "=D"] : []),
};

describe("buildEmojiData", () => {
  const out = buildEmojiData(input);

  it("keeps emoji presentation in shown groups, sorted by order", () => {
    assert.deepEqual(out.emojis.map((e) => e.hexcode), ["1F600", "1F603", "1F44B"]);
    assert.deepEqual(out.groups, [{ id: 0, key: "smileys-emotion", name: "smileys & emotion" }]);
    assert.equal(out.format, EMOJI_DATA_FORMAT);
  });

  it("merges shortcodes case-insensitively and emoticons exactly", () => {
    const grin = out.emojis[0];
    assert.deepEqual(grin.shortcodes, ["grinning", "Grin"]);
    assert.deepEqual(grin.emoticons, [":D", "=D"]);
    assert.deepEqual(grin.keywords, []);
    assert.equal(grin.skins, undefined);
  });

  it("keeps skin-tone variants", () => {
    assert.deepEqual(out.emojis[2].skins, [
      { emoji: "👋🏻", tone: 1 },
      { emoji: "👋🏼", tone: [2, 3] },
    ]);
  });

  it("serialises to JSON with one emoji per line", () => {
    const text = serializeEmojiData(out);
    assert.deepEqual(JSON.parse(text), out);
    assert.equal(text.split("\n").filter((l) => l.includes('"hexcode"')).length, 3);
  });
});

describe("src/data/emoji.json", () => {
  const data = JSON.parse(
    readFileSync(new URL("../data/emoji.json", import.meta.url), "utf8"),
  );

  it("is the current format with every field the provider needs", () => {
    assert.equal(data.format, EMOJI_DATA_FORMAT);
    assert.ok(data.emojis.length > 1000);
    for (const e of data.emojis) {
      assert.equal(typeof e.emoji, "string");
      assert.match(e.hexcode, /^[0-9A-F]+(-[0-9A-F]+)*$/);
      assert.ok(e.name.length > 0);
      assert.ok(data.groups.some((g: { id: number }) => g.id === e.group));
    }
  });
});
