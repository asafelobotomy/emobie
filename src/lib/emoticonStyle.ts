import type { EmoticonStyle } from "../types/preferences";

/** Classic “nose” forms use a hyphen after the eyes (e.g. :-) ;-) ). */
export function isClassicNoseEmoticon(emoticon: string): boolean {
  return emoticon.includes(":-") || emoticon.includes(";-");
}

/** Minimal forms omit the nose hyphen (e.g. :) ;) ). */
export function isMinimalSmileyEmoticon(emoticon: string): boolean {
  if (isClassicNoseEmoticon(emoticon)) return false;
  return /^[:;=8][\)\(\[\]DPpOo\/\\|@#cCsSlLzZ3><']/.test(emoticon);
}

export function toMinimalVariant(emoticon: string): string | null {
  if (emoticon.includes(":-")) return emoticon.replace(":-", ":");
  if (emoticon.includes(";-")) return emoticon.replace(";-", ";");
  return null;
}

export function toClassicVariant(emoticon: string): string | null {
  if (isClassicNoseEmoticon(emoticon)) return null;
  const match = emoticon.match(/^([=;8]?)([:;8])(.+)$/);
  if (!match) return null;
  return `${match[1]}${match[2]}-${match[3]}`;
}

/** Pick one nose style when both :) and :-) variants exist for the same emoji. */
export function filterEmoticonsByStyle(
  emoticons: string[],
  style: EmoticonStyle,
): string[] {
  const available = new Set(emoticons);
  const drop = new Set<string>();

  for (const emoticon of emoticons) {
    if (style === "classic") {
      const classic = toClassicVariant(emoticon);
      if (classic && available.has(classic) && isMinimalSmileyEmoticon(emoticon)) {
        drop.add(emoticon);
      }
      continue;
    }
    const minimal = toMinimalVariant(emoticon);
    if (minimal && available.has(minimal) && isClassicNoseEmoticon(emoticon)) {
      drop.add(emoticon);
    }
  }

  return emoticons.filter((emoticon) => !drop.has(emoticon));
}
