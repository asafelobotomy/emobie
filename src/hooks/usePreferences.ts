import { useCallback, useEffect, useState } from "react";
import { load, type Store } from "@tauri-apps/plugin-store";
import {
  DEFAULT_PREFERENCES,
  type Macro,
  type Preferences,
  type ThemeMode,
  type EmojiSize,
  type SortBy,
} from "../types/preferences";
import { findEmojiByChar, type SkinTone } from "../data/loadEmojis";
import { normalizePreferences } from "../lib/normalizePreferences";

const STORE_PATH = "emobie-preferences.json";

let storePromise: Promise<Store> | null = null;

function getStore(): Promise<Store> {
  if (!storePromise) {
    storePromise = load(STORE_PATH, { autoSave: true });
  }
  return storePromise;
}

async function readPreferences(): Promise<Preferences> {
  try {
    const store = await getStore();
    const saved = await store.get<Partial<Preferences>>("preferences");
    return normalizePreferences(saved);
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

async function writePreferences(prefs: Preferences): Promise<boolean> {
  try {
    const store = await getStore();
    await store.set("preferences", prefs);
    await store.save();
    return true;
  } catch (error) {
    console.error("Failed to save preferences", error);
    return false;
  }
}

export function usePreferences() {
  const [prefs, setPrefs] = useState<Preferences>(DEFAULT_PREFERENCES);
  const [ready, setReady] = useState(false);
  const [prefsError, setPrefsError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    readPreferences().then((loaded) => {
      if (!cancelled) {
        setPrefs(loaded);
        setReady(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const persist = useCallback((next: Preferences) => {
    void writePreferences(next).then((ok) => {
      setPrefsError(ok ? null : "Could not save preferences.");
    });
  }, []);

  const update = useCallback(
    (patch: Partial<Preferences>) => {
      setPrefs((current) => {
        const next = { ...current, ...patch };
        persist(next);
        return next;
      });
    },
    [persist],
  );

  const setTheme = useCallback(
    (theme: ThemeMode) => update({ theme }),
    [update],
  );
  const setPinned = useCallback(
    (pinned: boolean) => update({ pinned }),
    [update],
  );
  const setEmojiSize = useCallback(
    (emojiSize: EmojiSize) => update({ emojiSize }),
    [update],
  );
  const setRecentMax = useCallback(
    (recentMax: number) => {
      setPrefs((current) => {
        const next = {
          ...current,
          recentMax,
          recents: current.recents.slice(0, recentMax),
        };
        persist(next);
        return next;
      });
    },
    [persist],
  );
  const setSkinTone = useCallback(
    (skinTone: SkinTone) => update({ skinTone }),
    [update],
  );
  const setHotkey = useCallback(
    (hotkey: string) => update({ hotkey }),
    [update],
  );
  const setShowTitleBar = useCallback(
    (showTitleBar: boolean) => update({ showTitleBar }),
    [update],
  );
  const setLaunchOnStartup = useCallback(
    (launchOnStartup: boolean) => update({ launchOnStartup }),
    [update],
  );
  const setStartMinimizedToTray = useCallback(
    (startMinimizedToTray: boolean) => update({ startMinimizedToTray }),
    [update],
  );
  const setAllowMultipleInstances = useCallback(
    (allowMultipleInstances: boolean) => update({ allowMultipleInstances }),
    [update],
  );
  const setSortBy = useCallback(
    (sortBy: SortBy) => update({ sortBy }),
    [update],
  );
  const setShowShortcodeMacros = useCallback(
    (showShortcodeMacros: boolean) => update({ showShortcodeMacros }),
    [update],
  );
  const setAutoPasteOnCopy = useCallback(
    (autoPasteOnCopy: boolean) => update({ autoPasteOnCopy }),
    [update],
  );
  const setExpandAsYouType = useCallback(
    (expandAsYouType: boolean) => update({ expandAsYouType }),
    [update],
  );
  const setCheckUpdatesOnStartup = useCallback(
    (checkUpdatesOnStartup: boolean) => update({ checkUpdatesOnStartup }),
    [update],
  );
  const setDismissedUpdateVersion = useCallback(
    (dismissedUpdateVersion: string | null) =>
      update({ dismissedUpdateVersion }),
    [update],
  );
  const setInputHelperSetupSeen = useCallback(
    (inputHelperSetupSeen: boolean) => update({ inputHelperSetupSeen }),
    [update],
  );

  const upsertMacro = useCallback(
    (macro: Macro) => {
      setPrefs((current) => {
        const without = current.macros.filter((item) => item.id !== macro.id);
        const clash = without.some((item) => item.trigger === macro.trigger);
        if (clash) return current;
        const next = { ...current, macros: [...without, macro] };
        persist(next);
        return next;
      });
    },
    [persist],
  );

  const removeMacro = useCallback(
    (id: string) => {
      setPrefs((current) => {
        const next = {
          ...current,
          macros: current.macros.filter((item) => item.id !== id),
        };
        persist(next);
        return next;
      });
    },
    [persist],
  );

  const setMacros = useCallback(
    (macros: Macro[]) => update({ macros }),
    [update],
  );

  const pushRecent = useCallback(
    (emoji: string) => {
      setPrefs((current) => {
        const nextRecents = [
          emoji,
          ...current.recents.filter((item) => item !== emoji),
        ].slice(0, current.recentMax);

        const match = findEmojiByChar(emoji);
        let usageCounts = current.usageCounts;
        let firstUsedAt = current.firstUsedAt;
        if (match) {
          const now = Date.now();
          usageCounts = {
            ...current.usageCounts,
            [match.hexcode]: (current.usageCounts[match.hexcode] ?? 0) + 1,
          };
          firstUsedAt = {
            ...current.firstUsedAt,
            [match.hexcode]: current.firstUsedAt[match.hexcode] ?? now,
          };
        }

        const next = {
          ...current,
          recents: nextRecents,
          usageCounts,
          firstUsedAt,
        };
        persist(next);
        return next;
      });
    },
    [persist],
  );

  const clearRecents = useCallback(() => {
    update({ recents: [] });
  }, [update]);

  const clearUsageStats = useCallback(() => {
    update({ usageCounts: {}, firstUsedAt: {} });
  }, [update]);

  const toggleFavorite = useCallback(
    (hexcode: string) => {
      setPrefs((current) => {
        const exists = current.favorites.includes(hexcode);
        const favorites = exists
          ? current.favorites.filter((item) => item !== hexcode)
          : [hexcode, ...current.favorites];
        const next = { ...current, favorites };
        persist(next);
        return next;
      });
    },
    [persist],
  );

  return {
    prefs,
    ready,
    prefsError,
    setTheme,
    setPinned,
    setEmojiSize,
    setRecentMax,
    setSkinTone,
    setHotkey,
    setShowTitleBar,
    setLaunchOnStartup,
    setStartMinimizedToTray,
    setAllowMultipleInstances,
    setSortBy,
    setShowShortcodeMacros,
    setAutoPasteOnCopy,
    setExpandAsYouType,
    setCheckUpdatesOnStartup,
    setDismissedUpdateVersion,
    setInputHelperSetupSeen,
    upsertMacro,
    removeMacro,
    setMacros,
    pushRecent,
    clearRecents,
    clearUsageStats,
    toggleFavorite,
    update,
  };
}
