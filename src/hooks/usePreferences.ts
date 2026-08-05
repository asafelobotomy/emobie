import { useCallback, useEffect, useState } from "react";
import { load, type Store } from "@tauri-apps/plugin-store";
import {
  DEFAULT_PREFERENCES,
  type Preferences,
  type ThemeMode,
  type EmojiSize,
} from "../types/preferences";
import type { SkinTone } from "../data/loadEmojis";

const STORE_PATH = "emobie-preferences.json";

let storePromise: Promise<Store> | null = null;

function getStore(): Promise<Store> {
  if (!storePromise) {
    storePromise = load(STORE_PATH, { autoSave: true });
  }
  return storePromise;
}

function normalizePreferences(saved: Partial<Preferences> | undefined): Preferences {
  const merged = { ...DEFAULT_PREFERENCES, ...saved };
  return {
    ...merged,
    recents: Array.isArray(merged.recents) ? merged.recents.filter(Boolean) : [],
    favorites: Array.isArray(merged.favorites)
      ? merged.favorites.filter(Boolean)
      : [],
    recentMax: Math.min(96, Math.max(8, Number(merged.recentMax) || 32)),
  };
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

async function writePreferences(prefs: Preferences): Promise<void> {
  const store = await getStore();
  await store.set("preferences", prefs);
  await store.save();
}

export function usePreferences() {
  const [prefs, setPrefs] = useState<Preferences>(DEFAULT_PREFERENCES);
  const [ready, setReady] = useState(false);

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

  const update = useCallback((patch: Partial<Preferences>) => {
    setPrefs((current) => {
      const next = { ...current, ...patch };
      void writePreferences(next);
      return next;
    });
  }, []);

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
  const setRecentMax = useCallback((recentMax: number) => {
    setPrefs((current) => {
      const next = {
        ...current,
        recentMax,
        recents: current.recents.slice(0, recentMax),
      };
      void writePreferences(next);
      return next;
    });
  }, []);
  const setSkinTone = useCallback(
    (skinTone: SkinTone) => update({ skinTone }),
    [update],
  );
  const setHotkey = useCallback(
    (hotkey: string) => update({ hotkey }),
    [update],
  );

  const pushRecent = useCallback((emoji: string) => {
    setPrefs((current) => {
      const nextRecents = [
        emoji,
        ...current.recents.filter((item) => item !== emoji),
      ].slice(0, current.recentMax);
      const next = { ...current, recents: nextRecents };
      void writePreferences(next);
      return next;
    });
  }, []);

  const clearRecents = useCallback(() => {
    update({ recents: [] });
  }, [update]);

  const toggleFavorite = useCallback((hexcode: string) => {
    setPrefs((current) => {
      const exists = current.favorites.includes(hexcode);
      const favorites = exists
        ? current.favorites.filter((item) => item !== hexcode)
        : [hexcode, ...current.favorites];
      const next = { ...current, favorites };
      void writePreferences(next);
      return next;
    });
  }, []);

  return {
    prefs,
    ready,
    setTheme,
    setPinned,
    setEmojiSize,
    setRecentMax,
    setSkinTone,
    setHotkey,
    pushRecent,
    clearRecents,
    toggleFavorite,
    update,
  };
}
