import type { Emoji as EmojibaseEmoji } from "emojibase";
import { EMOJI } from "emojibase";
import data from "emojibase-data/en/data.json";
import messages from "emojibase-data/en/messages.json";
import shortcodes from "emojibase-data/en/shortcodes/emojibase.json";

export type SkinTone = 0 | 1 | 2 | 3 | 4 | 5;

export type EmobieEmoji = {
  emoji: string;
  label: string;
  hexcode: string;
  group: number;
  tags: string[];
  shortcodes: string[];
  skins?: { emoji: string; tone: number | number[] }[];
  order: number;
};

export type Category = {
  id: number;
  key: string;
  label: string;
  icon: string;
};

const CATEGORY_ICONS: Record<string, string> = {
  "smileys-emotion": "😊",
  "people-body": "🧑",
  "animals-nature": "🐻",
  "food-drink": "🍔",
  "travel-places": "✈️",
  activities: "⚽",
  objects: "💡",
  symbols: "❤️",
  flags: "🏁",
};

const SKIP_GROUPS = new Set(["component"]);

function normalizeShortcodes(value: string | string[] | undefined): string[] {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

function buildEmojis(): EmobieEmoji[] {
  const emojiData = data as EmojibaseEmoji[];
  const shortcodeMap = shortcodes as Record<string, string | string[]>;

  return emojiData
    .filter((item) => item.type === EMOJI && item.group !== undefined)
    .filter((item) => {
      const groupMeta = messages.groups.find((g) => g.order === item.group);
      return groupMeta && !SKIP_GROUPS.has(groupMeta.key);
    })
    .map((item) => ({
      emoji: item.emoji,
      label: item.label,
      hexcode: item.hexcode,
      group: item.group as number,
      tags: item.tags ?? [],
      shortcodes: normalizeShortcodes(shortcodeMap[item.hexcode]),
      skins: item.skins?.map((skin) => ({
        emoji: skin.emoji,
        tone: skin.tone as number | number[],
      })),
      order: item.order ?? 0,
    }))
    .sort((a, b) => a.order - b.order);
}

export const EMOJIS: EmobieEmoji[] = buildEmojis();

export const CATEGORIES: Category[] = messages.groups
  .filter((group) => !SKIP_GROUPS.has(group.key))
  .map((group) => ({
    id: group.order,
    key: group.key,
    label: group.message.replace(/\b\w/g, (c) => c.toUpperCase()),
    icon: CATEGORY_ICONS[group.key] ?? "✨",
  }));

export function applySkinTone(emoji: EmobieEmoji, tone: SkinTone): string {
  if (tone === 0 || !emoji.skins?.length) {
    return emoji.emoji;
  }

  const match = emoji.skins.find((skin) => {
    if (typeof skin.tone === "number") {
      return skin.tone === tone;
    }
    return skin.tone[0] === tone;
  });

  return match?.emoji ?? emoji.emoji;
}

export const FAVORITES_CATEGORY_ID = -1;

export const FAVORITES_CATEGORY: Category = {
  id: FAVORITES_CATEGORY_ID,
  key: "favorites",
  label: "Favorites",
  icon: "⭐",
};

export const NAV_CATEGORIES: Category[] = [FAVORITES_CATEGORY, ...CATEGORIES];

function sortFavoritesFirst(
  emojis: EmobieEmoji[],
  favoriteHexcodes: string[],
): EmobieEmoji[] {
  if (favoriteHexcodes.length === 0) return emojis;

  const favoriteSet = new Set(favoriteHexcodes);
  const favoriteOrder = new Map(
    favoriteHexcodes.map((hexcode, index) => [hexcode, index]),
  );

  return [...emojis].sort((a, b) => {
    const aFav = favoriteSet.has(a.hexcode);
    const bFav = favoriteSet.has(b.hexcode);
    if (aFav && bFav) {
      return (
        (favoriteOrder.get(a.hexcode) ?? 0) - (favoriteOrder.get(b.hexcode) ?? 0)
      );
    }
    if (aFav) return -1;
    if (bFav) return 1;
    return a.order - b.order;
  });
}

export type EmojiSortBy = "default" | "name" | "type" | "dateAdded" | "uses";

export type EmojiSortContext = {
  sortBy: EmojiSortBy;
  usageCounts: Record<string, number>;
  firstUsedAt: Record<string, number>;
};

function compareBySort(
  a: EmobieEmoji,
  b: EmobieEmoji,
  ctx: EmojiSortContext,
): number {
  switch (ctx.sortBy) {
    case "name":
      return a.label.localeCompare(b.label, undefined, { sensitivity: "base" });
    case "type":
      return a.group - b.group || a.order - b.order;
    case "dateAdded": {
      const aAt = ctx.firstUsedAt[a.hexcode] ?? Number.POSITIVE_INFINITY;
      const bAt = ctx.firstUsedAt[b.hexcode] ?? Number.POSITIVE_INFINITY;
      if (aAt !== bAt) return aAt - bAt;
      return a.order - b.order;
    }
    case "uses": {
      const aUses = ctx.usageCounts[a.hexcode] ?? 0;
      const bUses = ctx.usageCounts[b.hexcode] ?? 0;
      if (aUses !== bUses) return bUses - aUses;
      return a.order - b.order;
    }
    case "default":
      return a.order - b.order;
    default: {
      const _exhaustive: never = ctx.sortBy;
      return _exhaustive;
    }
  }
}

function applySort(
  emojis: EmobieEmoji[],
  favoriteHexcodes: string[],
  ctx: EmojiSortContext,
): EmobieEmoji[] {
  if (ctx.sortBy === "default") {
    return sortFavoritesFirst(emojis, favoriteHexcodes);
  }

  return [...emojis].sort((a, b) => compareBySort(a, b, ctx));
}

export function searchEmojis(
  query: string,
  tone: SkinTone,
  favoriteHexcodes: string[] = [],
  sortCtx: EmojiSortContext = {
    sortBy: "default",
    usageCounts: {},
    firstUsedAt: {},
  },
): EmobieEmoji[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];

  const matches = EMOJIS.filter((emoji) => {
    if (emoji.label.toLowerCase().includes(q)) return true;
    if (emoji.tags.some((tag) => tag.includes(q))) return true;
    if (emoji.shortcodes.some((code) => code.toLowerCase().includes(q))) return true;
    return false;
  }).map((emoji) => ({
    ...emoji,
    emoji: applySkinTone(emoji, tone),
  }));

  return applySort(matches, favoriteHexcodes, sortCtx);
}

export function emojisForCategory(
  groupId: number,
  tone: SkinTone,
  favoriteHexcodes: string[] = [],
  sortCtx: EmojiSortContext = {
    sortBy: "default",
    usageCounts: {},
    firstUsedAt: {},
  },
): EmobieEmoji[] {
  if (groupId === FAVORITES_CATEGORY_ID) {
    const favorites = favoriteHexcodes
      .map((hexcode) => EMOJIS.find((emoji) => emoji.hexcode === hexcode))
      .filter((emoji): emoji is EmobieEmoji => Boolean(emoji))
      .map((emoji) => ({
        ...emoji,
        emoji: applySkinTone(emoji, tone),
      }));

    if (sortCtx.sortBy === "default") {
      return favorites;
    }
    return [...favorites].sort((a, b) => compareBySort(a, b, sortCtx));
  }

  const categoryEmojis = EMOJIS.filter((emoji) => emoji.group === groupId).map(
    (emoji) => ({
      ...emoji,
      emoji: applySkinTone(emoji, tone),
    }),
  );

  return applySort(categoryEmojis, favoriteHexcodes, sortCtx);
}

export function findEmojiByChar(char: string): EmobieEmoji | undefined {
  return EMOJIS.find(
    (emoji) =>
      emoji.emoji === char ||
      emoji.skins?.some((skin) => skin.emoji === char),
  );
}
