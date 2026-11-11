// Generates src/data/emoji.json (installed to /usr/share/emobie/emoji.json) from the
// emojibase data in node_modules. The app imports it and emobie-provider reads the
// installed copy, so both search the same data.
//
//   node --experimental-strip-types scripts/build-emoji-json.ts           write the file
//   node --experimental-strip-types scripts/build-emoji-json.ts --check   exit 1 if stale
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { buildEmojiData, serializeEmojiData } from "../src/lib/buildEmojiData.ts";
import { extraEmoticonsFor } from "../src/data/extraEmoticons.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "src/data/emoji.json");
const json = (path: string) => JSON.parse(readFileSync(join(root, "node_modules", path), "utf8"));

const pkg = json("emojibase-data/package.json");
const text = serializeEmojiData(
  buildEmojiData({
    data: json("emojibase-data/en/data.json"),
    messages: json("emojibase-data/en/messages.json"),
    shortcodePacks: [
      json("emojibase-data/en/shortcodes/emojibase.json"),
      json("emojibase-data/en/shortcodes/github.json"),
      json("emojibase-data/en/shortcodes/emojibase-legacy.json"),
    ],
    extraEmoticonsFor,
    source: `emojibase-data ${pkg.version}`,
  }),
);

if (process.argv.includes("--check")) {
  let current = "";
  try {
    current = readFileSync(out, "utf8");
  } catch {
    // missing counts as stale
  }
  if (current !== text) {
    console.error(
      "src/data/emoji.json is out of date with emojibase-data or src/data/extraEmoticons.ts.\n" +
        "Run: npm run emoji:build",
    );
    process.exit(1);
  }
  console.log(`src/data/emoji.json is up to date (emojibase-data ${pkg.version})`);
} else {
  writeFileSync(out, text);
  console.log(`Wrote src/data/emoji.json (emojibase-data ${pkg.version})`);
}
