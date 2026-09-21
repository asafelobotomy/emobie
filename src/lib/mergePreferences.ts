import type { Macro, Preferences } from "../types/preferences";
import { normalizeMacros } from "./normalizePreferences.ts";

/**
 * Combine the active store with the other on-disk snapshots.
 *
 * The primary store is authoritative for every user list/map it already has —
 * even an empty one — so deleting a macro/favorite/recent, clearing recents, or
 * clearing usage stats survives a restart. Other snapshots (durable mirror,
 * legacy or other-install stores) only *recover* keys the primary lacks, which
 * is the fresh-install / migration case. A plain union would resurrect anything
 * a stale snapshot still contained.
 */
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

  const recovery = extras.filter((item): item is Partial<Preferences> =>
    Boolean(item),
  );
  const own = primary ?? {};

  base.macros = Array.isArray(own.macros)
    ? mergeMacros(own.macros)
    : mergeMacros(
        recovery.flatMap((source) =>
          Array.isArray(source.macros) ? source.macros : [],
        ),
      );
  base.favorites = Array.isArray(own.favorites)
    ? mergeUnique(own.favorites)
    : mergeUnique(
        recovery.flatMap((source) =>
          Array.isArray(source.favorites) ? source.favorites : [],
        ),
      );
  base.recents = Array.isArray(own.recents)
    ? mergeUnique(own.recents)
    : mergeUnique(
        recovery.flatMap((source) =>
          Array.isArray(source.recents) ? source.recents : [],
        ),
      );
  base.usageCounts = isCountMap(own.usageCounts)
    ? mergeMaxMaps([own.usageCounts])
    : mergeMaxMaps(recovery.map((source) => source.usageCounts));
  base.firstUsedAt = isCountMap(own.firstUsedAt)
    ? mergeMinMaps([own.firstUsedAt])
    : mergeMinMaps(recovery.map((source) => source.firstUsedAt));
  return base;
}

function isCountMap(value: unknown): value is Record<string, number> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
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
