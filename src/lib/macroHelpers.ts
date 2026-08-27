import type { Macro, MacroTriggerMode } from "../types/preferences.ts";

export type MacroEntry = Macro & {
  source: "custom" | "shortcode";
  label?: string;
  /** Emojibase group id for shortcode macros. */
  group?: number;
};

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
      const expansion =
        mode === "space" && keepTriggerSpace && !macro.expansion.endsWith(" ")
          ? `${macro.expansion} `
          : macro.expansion;
      return {
        trigger: macro.trigger,
        expansion,
        mode,
      };
    });
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
