import { parse, stringify } from "yaml";
import type { Macro } from "../types/preferences.ts";
import { normalizeMacros } from "./normalizePreferences.ts";

/** Keep in sync with emobie-inputd state caps. */
const MAX_YAML_BYTES = 256 * 1024;
const MAX_MATCHES = 2_000;
const MAX_TRIGGER_LEN = 256;
const MAX_EXPANSION_LEN = 64 * 1024;
const MAX_TRIGGERS_PER_ENTRY = 32;

type YamlMatch = {
  trigger?: unknown;
  triggers?: unknown;
  replace?: unknown;
  hotkey?: unknown;
  enabled?: unknown;
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
    const entry: Record<string, string | boolean> = {
      trigger: macro.trigger,
      replace: macro.expansion,
    };
    if (macro.hotkey) entry.hotkey = macro.hotkey;
    if (!macro.enabled) entry.enabled = false;
    return entry;
  });
  return stringify({ matches });
}

export function importMacrosYaml(
  yamlText: string,
  existing: Macro[],
): { macros: Macro[]; imported: number; skipped: number } {
  if (yamlText.length > MAX_YAML_BYTES) {
    throw new Error(`YAML file is too large (max ${MAX_YAML_BYTES} bytes).`);
  }

  let parsed: unknown;
  try {
    parsed = parse(yamlText, { maxAliasCount: 32 });
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
  if (matchList.length > MAX_MATCHES) {
    throw new Error(`Too many matches (max ${MAX_MATCHES}).`);
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
    if (triggers.length > MAX_TRIGGERS_PER_ENTRY) {
      skipped += 1;
      continue;
    }
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
    if (expansion.length > MAX_EXPANSION_LEN) {
      skipped += 1;
      continue;
    }
    const hotkey =
      typeof item.hotkey === "string" && item.hotkey.trim()
        ? item.hotkey.trim()
        : null;
    const enabledFromYaml =
      item.enabled === undefined
        ? undefined
        : item.enabled !== false && item.enabled !== "false";

    for (const trigger of triggers) {
      if ([...trigger].length > MAX_TRIGGER_LEN) {
        skipped += 1;
        continue;
      }
      if (byTrigger.size >= MAX_MATCHES && !byTrigger.has(trigger)) {
        skipped += 1;
        continue;
      }
      const previous = byTrigger.get(trigger);
      byTrigger.set(trigger, {
        id: previous?.id ?? crypto.randomUUID(),
        trigger,
        expansion,
        hotkey: hotkey ?? previous?.hotkey ?? null,
        enabled: enabledFromYaml ?? previous?.enabled ?? true,
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
