import type { Macro, Preferences } from "../types/preferences";
import { normalizeMacros } from "./normalizePreferences.ts";

/** Merge user lists/stats from multiple preference sources (update-safe). */
export function mergePreferencePartials(
  primary: Partial<Preferences> | undefined,
  extras: Array<Partial<Preferences> | undefined>,
): Partial<Preferences> {
  const sources = [primary, ...extras].filter(
    (item): item is Partial<Preferences> => Boolean(item),
  );
  if (sources.length === 0) return {};

  const base: Partial<Preferences> = { ...sources[0] };
  for (const extra of sources.slice(1)) {
    // Prefer primary for scalar settings; fill only when primary omitted them.
    for (const [key, value] of Object.entries(extra)) {
      if (value === undefined) continue;
      const current = (base as Record<string, unknown>)[key];
      if (current === undefined || current === null) {
        (base as Record<string, unknown>)[key] = value;
      }
    }
  }

  base.macros = mergeMacros(
    sources.flatMap((source) =>
      Array.isArray(source.macros) ? source.macros : [],
    ),
  );
  base.favorites = mergeUnique(
    sources.flatMap((source) =>
      Array.isArray(source.favorites) ? source.favorites : [],
    ),
  );
  base.recents = mergeUnique(
    sources.flatMap((source) =>
      Array.isArray(source.recents) ? source.recents : [],
    ),
  );
  base.usageCounts = mergeMaxMaps(
    sources.map((source) => source.usageCounts),
  );
  base.firstUsedAt = mergeMinMaps(
    sources.map((source) => source.firstUsedAt),
  );
  return base;
}

function mergeMacros(macros: Macro[]): Macro[] {
  return normalizeMacros(macros);
}

function mergeUnique(values: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const value of values) {
    if (!value || seen.has(value)) continue;
    seen.add(value);
    out.push(value);
  }
  return out;
}

function mergeMaxMaps(
  maps: Array<Record<string, number> | undefined>,
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const map of maps) {
    if (!map) continue;
    for (const [key, raw] of Object.entries(map)) {
      const n = Number(raw);
      if (!key || !Number.isFinite(n) || n <= 0) continue;
      const next = Math.floor(n);
      out[key] = Math.max(out[key] ?? 0, next);
    }
  }
  return out;
}

function mergeMinMaps(
  maps: Array<Record<string, number> | undefined>,
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const map of maps) {
    if (!map) continue;
    for (const [key, raw] of Object.entries(map)) {
      const n = Number(raw);
      if (!key || !Number.isFinite(n) || n <= 0) continue;
      const next = Math.floor(n);
      out[key] = out[key] === undefined ? next : Math.min(out[key], next);
    }
  }
  return out;
}

export function userDataFingerprint(prefs: {
  macros?: Macro[];
  favorites?: string[];
  recents?: string[];
  usageCounts?: Record<string, number>;
  firstUsedAt?: Record<string, number>;
  expandAsYouType?: boolean;
  expandTriggerMode?: string;
  expandKeepTriggerSpace?: boolean;
  expandRestoreClipboard?: boolean;
  inputHelperSetupSeen?: boolean;
}): string {
  return JSON.stringify({
    macros: prefs.macros ?? [],
    favorites: prefs.favorites ?? [],
    recents: prefs.recents ?? [],
    usageCounts: prefs.usageCounts ?? {},
    firstUsedAt: prefs.firstUsedAt ?? {},
    expandAsYouType: Boolean(prefs.expandAsYouType),
    expandTriggerMode: prefs.expandTriggerMode ?? "space",
    expandKeepTriggerSpace: Boolean(prefs.expandKeepTriggerSpace),
    expandRestoreClipboard: Boolean(prefs.expandRestoreClipboard),
    inputHelperSetupSeen: Boolean(prefs.inputHelperSetupSeen),
  });
}
