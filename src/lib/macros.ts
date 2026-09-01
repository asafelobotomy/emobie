import {
  applySkinTone,
  EMOJIS,
  type SkinTone,
} from "../data/loadEmojis";
import type { EmoticonStyle, Macro } from "../types/preferences";
import { filterEmoticonsByStyle } from "./emoticonStyle";
import {
  shortcodeTrigger,
  type MacroEntry,
} from "./macroHelpers";

export type { MacroEntry } from "./macroHelpers";
export {
  shortcodeTrigger,
  searchMacros,
  expansionMatches,
  customExpansionMatches,
  findHotkeyConflict,
  findTriggerConflict,
} from "./macroHelpers";

function pushUnique(
  entries: MacroEntry[],
  seen: Set<string>,
  entry: MacroEntry,
): void {
  if (!entry.trigger || seen.has(entry.trigger)) return;
  seen.add(entry.trigger);
  entries.push(entry);
}

const EMOJI_BY_HEX = new Map(EMOJIS.map((emoji) => [emoji.hexcode, emoji]));

/** Shortcodes and emoticons for favorited emojis only (not the full library). */
export function buildFavoriteEmojiMacros(
  favorites: string[],
  skinTone: SkinTone,
  emoticonStyle: EmoticonStyle,
): MacroEntry[] {
  const entries: MacroEntry[] = [];
  const seen = new Set<string>();

  for (const hexcode of favorites) {
    const emoji = EMOJI_BY_HEX.get(hexcode);
    if (!emoji) continue;
    const expansion = applySkinTone(emoji, skinTone);
    for (const code of emoji.shortcodes) {
      pushUnique(entries, seen, {
        id: `favorite:${hexcode}:${shortcodeTrigger(code)}`,
        trigger: shortcodeTrigger(code),
        expansion,
        hotkey: null,
        enabled: true,
        source: "favorite",
        label: emoji.label,
        group: emoji.group,
      });
    }
    for (const emoticon of filterEmoticonsByStyle(
      emoji.emoticons,
      emoticonStyle,
    )) {
      pushUnique(entries, seen, {
        id: `favorite:${hexcode}:emoticon:${emoticon}`,
        trigger: emoticon,
        expansion,
        hotkey: null,
        enabled: true,
        source: "favorite",
        label: emoji.label,
        group: emoji.group,
      });
    }
  }

  return entries;
}

export function mergeMacros(
  custom: Macro[],
  options: {
    favoriteEmojiMacros: boolean;
    favorites: string[];
    skinTone: SkinTone;
    emoticonStyle: EmoticonStyle;
  },
): MacroEntry[] {
  const customEntries: MacroEntry[] = custom.map((macro) => ({
    ...macro,
    source: "custom",
  }));
  if (!options.favoriteEmojiMacros || options.favorites.length === 0) {
    return customEntries;
  }

  const customTriggers = new Set(custom.map((macro) => macro.trigger));
  const favoriteEntries = buildFavoriteEmojiMacros(
    options.favorites,
    options.skinTone,
    options.emoticonStyle,
  ).filter((entry) => !customTriggers.has(entry.trigger));
  return [...customEntries, ...favoriteEntries];
}
