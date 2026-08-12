import { parse, stringify } from "yaml";
import type { Macro } from "../types/preferences.ts";
import { normalizeMacros } from "./normalizePreferences.ts";

type YamlMatch = {
  trigger?: unknown;
  triggers?: unknown;
  replace?: unknown;
  hotkey?: unknown;
};

function asStringList(value: unknown): string[] {
  if (typeof value === "string" && value.trim()) return [value.trim()];
  if (Array.isArray(value)) {
    return value
      .filter((item): item is string => typeof item === "string")
      .map((item) => item.trim())
      .filter(Boolean);
  }
  return [];
}

export function exportMacrosYaml(macros: Macro[]): string {
  const matches = macros.map((macro) => {
    const entry: Record<string, string> = {
      trigger: macro.trigger,
      replace: macro.expansion,
    };
    if (macro.hotkey) entry.hotkey = macro.hotkey;
    return entry;
  });
  return stringify({ matches });
}

export function importMacrosYaml(
  yamlText: string,
  existing: Macro[],
): { macros: Macro[]; imported: number; skipped: number } {
  let parsed: unknown;
  try {
    parsed = parse(yamlText);
  } catch {
    throw new Error("Invalid YAML.");
  }

  const root =
    parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;
  const matchList = Array.isArray(root?.matches)
    ? (root.matches as YamlMatch[])
    : Array.isArray(parsed)
      ? (parsed as YamlMatch[])
      : null;

  if (!matchList) {
    throw new Error('YAML must contain a "matches" list.');
  }

  const byTrigger = new Map(existing.map((macro) => [macro.trigger, macro]));
  let imported = 0;
  let skipped = 0;

  for (const item of matchList) {
    if (!item || typeof item !== "object") {
      skipped += 1;
      continue;
    }
    const triggers = [
      ...asStringList(item.trigger),
      ...asStringList(item.triggers),
    ];
    const expansion =
      typeof item.replace === "string"
        ? item.replace
        : typeof item.replace === "number"
          ? String(item.replace)
          : "";
    if (triggers.length === 0 || expansion.length === 0) {
      skipped += 1;
      continue;
    }
    const hotkey =
      typeof item.hotkey === "string" && item.hotkey.trim()
        ? item.hotkey.trim()
        : null;

    for (const trigger of triggers) {
      const previous = byTrigger.get(trigger);
      byTrigger.set(trigger, {
        id: previous?.id ?? crypto.randomUUID(),
        trigger,
        expansion,
        hotkey: hotkey ?? previous?.hotkey ?? null,
        enabled: previous?.enabled ?? true,
      });
      imported += 1;
    }
  }

  return {
    macros: normalizeMacros([...byTrigger.values()]),
    imported,
    skipped,
  };
}
