import {
  applySkinTone,
  EMOJIS,
  type SkinTone,
} from "../data/loadEmojis";
import type { Macro } from "../types/preferences";
import {
  shortcodeTrigger,
  type MacroEntry,
} from "./macroHelpers";

export type { MacroEntry } from "./macroHelpers";
export {
  shortcodeTrigger,
  searchMacros,
  expansionMatches,
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

export function buildShortcodeMacros(skinTone: SkinTone): MacroEntry[] {
  const entries: MacroEntry[] = [];
  const seen = new Set<string>();

  for (const emoji of EMOJIS) {
    const expansion = applySkinTone(emoji, skinTone);
    for (const code of emoji.shortcodes) {
      pushUnique(entries, seen, {
        id: `shortcode:${shortcodeTrigger(code)}`,
        trigger: shortcodeTrigger(code),
        expansion,
        hotkey: null,
        enabled: true,
        source: "shortcode",
        label: emoji.label,
      });
    }
    for (const emoticon of emoji.emoticons) {
      pushUnique(entries, seen, {
        id: `emoticon:${emoticon}`,
        trigger: emoticon,
        expansion,
        hotkey: null,
        enabled: true,
        source: "shortcode",
        label: emoji.label,
      });
    }
  }

  return entries;
}

export function mergeMacros(
  custom: Macro[],
  options: { showShortcodes: boolean; skinTone: SkinTone },
): MacroEntry[] {
  const customEntries: MacroEntry[] = custom.map((macro) => ({
    ...macro,
    source: "custom",
  }));
  if (!options.showShortcodes) return customEntries;

  const customTriggers = new Set(custom.map((macro) => macro.trigger));
  const shortcodes = buildShortcodeMacros(options.skinTone).filter(
    (entry) => !customTriggers.has(entry.trigger),
  );
  return [...customEntries, ...shortcodes];
}
