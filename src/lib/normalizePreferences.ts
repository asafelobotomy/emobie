import {
  DEFAULT_PREFERENCES,
  type Macro,
  type Preferences,
  type ThemeMode,
  type EmojiSize,
  type SortBy,
} from "../types/preferences.ts";

type SkinTone = 0 | 1 | 2 | 3 | 4 | 5;

function normalizeCountMap(value: unknown): Record<string, number> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  const result: Record<string, number> = {};
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const n = Number(raw);
    if (key && Number.isFinite(n) && n > 0) {
      result[key] = Math.floor(n);
    }
  }
  return result;
}

function normalizeMacro(raw: unknown): Macro | null {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const item = raw as Record<string, unknown>;
  const id = typeof item.id === "string" ? item.id.trim() : "";
  const trigger = typeof item.trigger === "string" ? item.trigger.trim() : "";
  const expansion =
    typeof item.expansion === "string" ? item.expansion : "";
  if (!id || !trigger || expansion.length === 0) return null;
  const hotkeyRaw = item.hotkey;
  const hotkey =
    typeof hotkeyRaw === "string" && hotkeyRaw.trim()
      ? hotkeyRaw.trim()
      : null;
  return {
    id,
    trigger,
    expansion,
    hotkey,
    enabled: item.enabled !== false,
  };
}

export function normalizeMacros(value: unknown): Macro[] {
  if (!Array.isArray(value)) return [];
  const seenIds = new Set<string>();
  const seenTriggers = new Set<string>();
  const macros: Macro[] = [];
  for (const entry of value) {
    const macro = normalizeMacro(entry);
    if (!macro) continue;
    if (seenIds.has(macro.id) || seenTriggers.has(macro.trigger)) continue;
    seenIds.add(macro.id);
    seenTriggers.add(macro.trigger);
    macros.push(macro);
  }
  return macros;
}

const THEME_VALUES = new Set<ThemeMode>(["system", "light", "dark"]);
const EMOJI_SIZE_VALUES = new Set<EmojiSize>(["sm", "md", "lg"]);
const SORT_VALUES = new Set<SortBy>([
  "default",
  "name",
  "type",
  "dateAdded",
  "uses",
]);

export function normalizePreferences(
  saved: Partial<Preferences> | undefined,
): Preferences {
  const merged = { ...DEFAULT_PREFERENCES, ...saved };
  const sortBy = SORT_VALUES.has(merged.sortBy as SortBy)
    ? (merged.sortBy as SortBy)
    : DEFAULT_PREFERENCES.sortBy;
  const theme = THEME_VALUES.has(merged.theme as ThemeMode)
    ? (merged.theme as ThemeMode)
    : DEFAULT_PREFERENCES.theme;
  const emojiSize = EMOJI_SIZE_VALUES.has(merged.emojiSize as EmojiSize)
    ? (merged.emojiSize as EmojiSize)
    : DEFAULT_PREFERENCES.emojiSize;
  const skinToneRaw = Number(merged.skinTone);
  const skinTone = (Number.isFinite(skinToneRaw)
    ? Math.min(5, Math.max(0, Math.round(skinToneRaw)))
    : DEFAULT_PREFERENCES.skinTone) as SkinTone;
  const hotkey =
    typeof merged.hotkey === "string" && merged.hotkey.trim()
      ? merged.hotkey.trim()
      : DEFAULT_PREFERENCES.hotkey;

  return {
    ...merged,
    theme,
    emojiSize,
    skinTone,
    hotkey,
    showTitleBar: Boolean(merged.showTitleBar),
    launchOnStartup: Boolean(merged.launchOnStartup),
    startMinimizedToTray: Boolean(merged.startMinimizedToTray),
    allowMultipleInstances: Boolean(merged.allowMultipleInstances),
    sortBy,
    usageCounts: normalizeCountMap(merged.usageCounts),
    firstUsedAt: normalizeCountMap(merged.firstUsedAt),
    recents: Array.isArray(merged.recents) ? merged.recents.filter(Boolean) : [],
    favorites: Array.isArray(merged.favorites)
      ? merged.favorites.filter(Boolean)
      : [],
    macros: normalizeMacros(merged.macros),
    showShortcodeMacros: merged.showShortcodeMacros !== false,
    autoPasteOnCopy: Boolean(merged.autoPasteOnCopy),
    expandAsYouType: Boolean(merged.expandAsYouType),
    checkUpdatesOnStartup: merged.checkUpdatesOnStartup !== false,
    dismissedUpdateVersion:
      typeof merged.dismissedUpdateVersion === "string" &&
      merged.dismissedUpdateVersion.trim()
        ? merged.dismissedUpdateVersion.trim()
        : null,
    inputHelperSetupSeen: Boolean(merged.inputHelperSetupSeen),
    recentMax: Math.min(96, Math.max(8, Number(merged.recentMax) || 32)),
  };
}
