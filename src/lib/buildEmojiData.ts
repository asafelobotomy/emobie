/**
 * Builds emobie's shared emoji data (src/data/emoji.json, installed to
 * /usr/share/emobie/emoji.json) from emojibase. The app and emobie-provider
 * both read that file, so their search results match.
 *
 * Pure: the caller passes the emojibase files in. Run
 * `npm run emoji:build` to regenerate and `npm run emoji:check` to verify.
 */

/** Bump when the shape changes; emobie-provider refuses other formats. */
export const EMOJI_DATA_FORMAT = 1;

/** Emojibase groups the app does not show (skin-tone swatches, hair parts). */
const SKIP_GROUPS = new Set(["component"]);

export type SourceEmoji = {
  emoji: string;
  label: string;
  hexcode: string;
  type?: number;
  group?: number;
  order?: number;
  tags?: string[];
  emoticon?: string | string[];
  skins?: { emoji: string; tone?: number | number[] }[];
};

export type SourceMessages = {
  groups: { key: string; message: string; order: number }[];
};

export type ShortcodePack = Record<string, string | string[]>;

export type EmojiGroup = { id: number; key: string; name: string };

export type EmojiSkin = { emoji: string; tone: number | number[] };

export type EmojiRecord = {
  emoji: string;
  /** Codepoints, emojibase style (`1F44D-1F3FB`). Favourites store this. */
  hexcode: string;
  name: string;
  group: number;
  order: number;
  keywords: string[];
  shortcodes: string[];
  emoticons: string[];
  skins?: EmojiSkin[];
};

export type EmojiData = {
  format: number;
  source: string;
  groups: EmojiGroup[];
  emojis: EmojiRecord[];
};

export type BuildInput = {
  data: SourceEmoji[];
  messages: SourceMessages;
  /** In priority order; duplicates (case-insensitive) are dropped. */
  shortcodePacks: ShortcodePack[];
  extraEmoticonsFor: (emoji: string) => string[];
  /** e.g. "emojibase-data 17.0.0". */
  source: string;
};

/** emojibase's `type` for fully-qualified emoji presentation. */
const EMOJI_TYPE = 1;

function asList(value: string | string[] | undefined): string[] {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

function uniqueBy(items: string[], key: (s: string) => string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const item of items) {
    if (!item) continue;
    const k = key(item);
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(item);
  }
  return out;
}

export function buildEmojiData(input: BuildInput): EmojiData {
  const skipped = new Set(
    input.messages.groups.filter((g) => SKIP_GROUPS.has(g.key)).map((g) => g.order),
  );
  const known = new Set(input.messages.groups.map((g) => g.order));

  const groups = input.messages.groups
    .filter((g) => !SKIP_GROUPS.has(g.key))
    .map((g) => ({ id: g.order, key: g.key, name: g.message }));

  const emojis = input.data
    .filter((item) => item.type === EMOJI_TYPE && item.group !== undefined)
    .filter((item) => known.has(item.group as number) && !skipped.has(item.group as number))
    .map((item): EmojiRecord => {
      const record: EmojiRecord = {
        emoji: item.emoji,
        hexcode: item.hexcode,
        name: item.label,
        group: item.group as number,
        order: item.order ?? 0,
        keywords: item.tags ?? [],
        shortcodes: uniqueBy(
          input.shortcodePacks.flatMap((pack) => asList(pack[item.hexcode])),
          (s) => s.toLowerCase(),
        ),
        emoticons: uniqueBy(
          [...asList(item.emoticon), ...input.extraEmoticonsFor(item.emoji)],
          (s) => s,
        ),
      };
      if (item.skins?.length) {
        record.skins = item.skins.map((skin) => ({
          emoji: skin.emoji,
          tone: skin.tone ?? 0,
        }));
      }
      return record;
    })
    .sort((a, b) => a.order - b.order);

  return { format: EMOJI_DATA_FORMAT, source: input.source, groups, emojis };
}

/** One emoji per line, so regenerating after an emojibase bump gives a readable diff. */
export function serializeEmojiData(data: EmojiData): string {
  const head = [
    `  "format": ${JSON.stringify(data.format)}`,
    `  "source": ${JSON.stringify(data.source)}`,
    `  "groups": [\n${data.groups.map((g) => `    ${JSON.stringify(g)}`).join(",\n")}\n  ]`,
    `  "emojis": [\n${data.emojis.map((e) => `    ${JSON.stringify(e)}`).join(",\n")}\n  ]`,
  ];
  return `{\n${head.join(",\n")}\n}\n`;
}
