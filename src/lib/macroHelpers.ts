import type { Macro, MacroTriggerMode } from "../types/preferences.ts";
import { applyFontStyle } from "./fontStyles.ts";

export type MacroEntry = Macro & {
  source: "custom" | "favorite";
  label?: string;
  /** Emojibase group id for favorite emoji macros. */
  group?: number;
};

/** The text a macro actually outputs: its expansion with its font style applied. */
export function macroOutput(macro: Pick<Macro, "expansion" | "fontStyle">): string {
  return applyFontStyle(macro.expansion, macro.fontStyle);
}

export function shortcodeTrigger(code: string): string {
  const trimmed = code.trim();
  if (!trimmed) return "";
  if (trimmed.startsWith(":") && trimmed.endsWith(":")) return trimmed;
  return `:${trimmed}:`;
}

export function searchMacros(macros: MacroEntry[], query: string): MacroEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return macros;
  return macros.filter((macro) => {
    if (macro.trigger.toLowerCase().includes(q)) return true;
    if (macro.expansion.toLowerCase().includes(q)) return true;
    if (macro.label?.toLowerCase().includes(q)) return true;
    return false;
  });
}

export function expansionMatches(
  macros: MacroEntry[],
  mode: MacroTriggerMode,
  keepTriggerSpace = false,
): { trigger: string; expansion: string; mode: MacroTriggerMode }[] {
  return macros
    .filter((macro) => macro.enabled)
    .map((macro) => {
      const output = macroOutput(macro);
      const expansion =
        mode === "space" && keepTriggerSpace && !output.endsWith(" ")
          ? `${output} `
          : output;
      return {
        trigger: macro.trigger,
        expansion,
        mode,
      };
    });
}

/** Matches for emobie-inputd — custom macros plus optional favorited emoji subset. */
export function customExpansionMatches(
  macros: MacroEntry[],
  mode: MacroTriggerMode,
  keepTriggerSpace = false,
): { trigger: string; expansion: string; mode: MacroTriggerMode }[] {
  return fitForHelper(
    expansionMatches(
      macros.filter(
        (macro) => macro.source === "custom" || macro.source === "favorite",
      ),
      mode,
      keepTriggerSpace,
    ),
  ).matches;
}

// Keep in sync with emobie-inputd: state.rs MAX_MATCHES / MAX_TRIGGER_LEN /
// MAX_EXPANSION_LEN and rpc.rs MAX_REQUEST_BYTES. The helper refuses a whole
// sync when one match breaks a limit, so one over-long macro would turn Expand
// off for every macro: leave such matches out instead.
export const HELPER_MAX_MATCHES = 2_000;
export const HELPER_MAX_TRIGGER_CHARS = 256;
export const HELPER_MAX_EXPANSION_BYTES = 64 * 1024;
/** The request limit is 512 KiB; keep room for the envelope. */
const HELPER_MAX_SYNC_BYTES = 500 * 1024;
// eslint-disable-next-line no-control-regex
const CONTROL_CHARS = /[\u0000-\u001f\u007f-\u009f]/;
const utf8 = new TextEncoder();

/** Why the helper would refuse this match, or null when it takes it. */
export function helperRefusal(trigger: string, expansion: string): string | null {
  if (!trigger || CONTROL_CHARS.test(trigger)) return "The trigger can't be empty or hold control characters.";
  if ([...trigger].length > HELPER_MAX_TRIGGER_CHARS) {
    return `A trigger is at most ${HELPER_MAX_TRIGGER_CHARS} characters.`;
  }
  if (!expansion || expansion.includes("\u0000")) return "The output can't be empty or hold a NUL character.";
  if (utf8.encode(expansion).length > HELPER_MAX_EXPANSION_BYTES) {
    return `The output is at most ${HELPER_MAX_EXPANSION_BYTES / 1024} KB (styled letters count up to 4 bytes each).`;
  }
  return null;
}

/** The matches the helper accepts, in order, within its count and size limits; `skipped` counts the rest. */
export function fitForHelper<T extends { trigger: string; expansion: string }>(
  matches: T[],
): { matches: T[]; skipped: number } {
  const kept: T[] = [];
  let bytes = 0;
  for (const match of matches) {
    if (kept.length >= HELPER_MAX_MATCHES) break;
    if (helperRefusal(match.trigger, match.expansion)) continue;
    const size = utf8.encode(JSON.stringify(match)).length + 1;
    if (bytes + size > HELPER_MAX_SYNC_BYTES) continue;
    bytes += size;
    kept.push(match);
  }
  return { matches: kept, skipped: matches.length - kept.length };
}

export function findHotkeyConflict(
  macros: Macro[],
  summonHotkey: string,
  candidate: string | null,
  excludeId?: string,
): string | null {
  if (!candidate) return null;
  if (candidate === summonHotkey) {
    return "Hotkey collides with the summon shortcut.";
  }
  const clash = macros.find(
    (macro) =>
      macro.id !== excludeId &&
      macro.hotkey === candidate &&
      macro.enabled,
  );
  if (clash) {
    return `Hotkey already used by ${clash.trigger}.`;
  }
  return null;
}

export function findTriggerConflict(
  macros: Macro[],
  trigger: string,
  excludeId?: string,
): string | null {
  const trimmed = trigger.trim();
  if (!trimmed) return "Trigger is required.";
  const clash = macros.find(
    (macro) => macro.id !== excludeId && macro.trigger === trimmed,
  );
  if (clash) return "Another macro already uses this trigger.";
  return null;
}
