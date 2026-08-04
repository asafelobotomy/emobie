import type { SkinTone } from "../data/loadEmojis";

export type ThemeMode = "system" | "light" | "dark";
export type EmojiSize = "sm" | "md" | "lg";

export type Preferences = {
  theme: ThemeMode;
  pinned: boolean;
  emojiSize: EmojiSize;
  recentMax: number;
  skinTone: SkinTone;
  hotkey: string;
  recents: string[];
  favorites: string[];
};

export const DEFAULT_PREFERENCES: Preferences = {
  theme: "system",
  pinned: false,
  emojiSize: "md",
  recentMax: 32,
  skinTone: 0,
  hotkey: "Control+Shift+Space",
  recents: [],
  favorites: [],
};

export const SKIN_TONES: { tone: SkinTone; label: string; swatch: string }[] = [
  { tone: 0, label: "Default", swatch: "#FFCC22" },
  { tone: 1, label: "Light", swatch: "#F7D7C4" },
  { tone: 2, label: "Medium-light", swatch: "#E2B496" },
  { tone: 3, label: "Medium", swatch: "#C68642" },
  { tone: 4, label: "Medium-dark", swatch: "#8D5524" },
  { tone: 5, label: "Dark", swatch: "#5C3317" },
];
