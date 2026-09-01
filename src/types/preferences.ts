import type { SkinTone } from "../data/loadEmojis";

export type ThemeMode = "system" | "light" | "dark";
export type EmojiSize = "sm" | "md" | "lg";
export type SortBy = "default" | "name" | "type" | "dateAdded" | "uses";

/** When expand-as-you-type is on: fire immediately, or only after Space. */
export type MacroTriggerMode = "immediate" | "space";

/** ASCII emoticon nose style for macros and triggers. */
export type EmoticonStyle = "minimal" | "classic";

export type Macro = {
  id: string;
  trigger: string;
  expansion: string;
  hotkey: string | null;
  enabled: boolean;
};

export type Preferences = {
  theme: ThemeMode;
  pinned: boolean;
  emojiSize: EmojiSize;
  recentMax: number;
  skinTone: SkinTone;
  hotkey: string;
  showTitleBar: boolean;
  launchOnStartup: boolean;
  startMinimizedToTray: boolean;
  /** When true, skip the single-instance lock so multiple windows can run. */
  allowMultipleInstances: boolean;
  sortBy: SortBy;
  /** hexcode -> copy count */
  usageCounts: Record<string, number>;
  /** hexcode -> first copy timestamp (ms) */
  firstUsedAt: Record<string, number>;
  recents: string[];
  favorites: string[];
  macros: Macro[];
  /** When on, shortcodes/emoticons for favorited emojis appear under Macros. */
  favoriteEmojiMacros: boolean;
  /** Prefer :) / :D or :-) / :-D style emoticon triggers. */
  emoticonStyle: EmoticonStyle;
  autoPasteOnCopy: boolean;
  expandAsYouType: boolean;
  /** How expand-as-you-type matches triggers (global). */
  expandTriggerMode: MacroTriggerMode;
  /** When trigger mode is Space: re-type a Space after the expansion. */
  expandKeepTriggerSpace: boolean;
  checkUpdatesOnStartup: boolean;
  dismissedUpdateVersion: string | null;
  /** True after the user finishes or skips first-run input helper setup. */
  inputHelperSetupSeen: boolean;
};

export const DEFAULT_PREFERENCES: Preferences = {
  theme: "system",
  pinned: false,
  emojiSize: "md",
  recentMax: 32,
  skinTone: 0,
  hotkey: "Control+Shift+Space",
  showTitleBar: false,
  launchOnStartup: false,
  startMinimizedToTray: false,
  allowMultipleInstances: false,
  sortBy: "default",
  usageCounts: {},
  firstUsedAt: {},
  recents: [],
  favorites: [],
  macros: [],
  favoriteEmojiMacros: false,
  emoticonStyle: "minimal",
  autoPasteOnCopy: false,
  expandAsYouType: false,
  expandTriggerMode: "space",
  expandKeepTriggerSpace: false,
  checkUpdatesOnStartup: true,
  dismissedUpdateVersion: null,
  inputHelperSetupSeen: false,
};

export const EMOTICON_STYLE_OPTIONS: {
  value: EmoticonStyle;
  label: string;
}[] = [
  { value: "minimal", label: ":) style" },
  { value: "classic", label: ":-) style" },
];

export const SORT_OPTIONS: { value: SortBy; label: string }[] = [
  { value: "default", label: "Default order" },
  { value: "name", label: "Name" },
  { value: "type", label: "Type (category)" },
  { value: "dateAdded", label: "First used" },
  { value: "uses", label: "Number of uses" },
];

export const SKIN_TONES: { tone: SkinTone; label: string; swatch: string }[] = [
  { tone: 0, label: "Default", swatch: "#FFCC22" },
  { tone: 1, label: "Light", swatch: "#F7D7C4" },
  { tone: 2, label: "Medium-light", swatch: "#E2B496" },
  { tone: 3, label: "Medium", swatch: "#C68642" },
  { tone: 4, label: "Medium-dark", swatch: "#8D5524" },
  { tone: 5, label: "Dark", swatch: "#5C3317" },
];
