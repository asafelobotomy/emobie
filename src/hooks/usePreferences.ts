import { useCallback, useEffect, useRef, useState } from "react";
import {
  DEFAULT_PREFERENCES,
  type Macro,
  type MacroTriggerMode,
  type EmoticonStyle,
  type Preferences,
  type ThemeMode,
  type EmojiSize,
  type SortBy,
  type PasteChordOverride,
} from "../types/preferences";
import { findEmojiByChar, type SkinTone } from "../data/loadEmojis";
import { readPreferences, writePreferences } from "../lib/preferencesIo";

export function usePreferences() {
  const [prefs, setPrefs] = useState<Preferences>(DEFAULT_PREFERENCES);
  const [ready, setReady] = useState(false);
  const [prefsError, setPrefsError] = useState<string | null>(null);
  const writeGeneration = useRef(0);
  const pendingWrite = useRef(Promise.resolve());
  // Latest committed preferences. Updates are computed from this ref rather
  // than inside a `setPrefs` updater, so persisting (a side effect) never runs
  // from a function React may invoke twice (StrictMode) or replay.
  const prefsRef = useRef<Preferences>(DEFAULT_PREFERENCES);

  useEffect(() => {
    let cancelled = false;
    readPreferences()
      .then((loaded) => {
        if (!cancelled) {
          prefsRef.current = loaded;
          setPrefs(loaded);
          setReady(true);
        }
      })
      .catch((error) => {
        console.error("Could not read preferences", error);
        if (!cancelled) {
          setPrefs(DEFAULT_PREFERENCES);
          setPrefsError(
            "Could not load saved preferences — using defaults. Check Settings for details.",
          );
          setReady(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const persist = useCallback((next: Preferences) => {
    // Monotonic across app launches: the durable mirror keeps the highest rev it
    // has seen and drops older writers, so a per-session counter that restarts
    // at 1 would have every write of a later session silently discarded.
    const generation = Math.max(writeGeneration.current + 1, Date.now());
    writeGeneration.current = generation;
    pendingWrite.current = pendingWrite.current.then(async () => {
      const ok = await writePreferences(next, generation);
      if (generation !== writeGeneration.current) return;
      setPrefsError(ok ? null : "Could not save preferences.");
    });
  }, []);

  const commit = useCallback(
    (next: Preferences) => {
      prefsRef.current = next;
      setPrefs(next);
      persist(next);
    },
    [persist],
  );

  /** Apply `fn` to the latest preferences; return the same object to skip. */
  const mutate = useCallback(
    (fn: (current: Preferences) => Preferences) => {
      const current = prefsRef.current;
      const next = fn(current);
      if (next !== current) commit(next);
    },
    [commit],
  );

  const update = useCallback(
    (patch: Partial<Preferences>) => mutate((current) => ({ ...current, ...patch })),
    [mutate],
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
    (recentMax: number) =>
      mutate((current) => ({
        ...current,
        recentMax,
        recents: current.recents.slice(0, recentMax),
      })),
    [mutate],
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
  const setFavoriteEmojiMacros = useCallback(
    (favoriteEmojiMacros: boolean) => update({ favoriteEmojiMacros }),
    [update],
  );
  const setEmoticonStyle = useCallback(
    (emoticonStyle: EmoticonStyle) => update({ emoticonStyle }),
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
  const setExpandTriggerMode = useCallback(
    (expandTriggerMode: MacroTriggerMode) => update({ expandTriggerMode }),
    [update],
  );
  const setExpandKeepTriggerSpace = useCallback(
    (expandKeepTriggerSpace: boolean) => update({ expandKeepTriggerSpace }),
    [update],
  );
  const setExpandRestoreClipboard = useCallback(
    (expandRestoreClipboard: boolean) => update({ expandRestoreClipboard }),
    [update],
  );
  const setPasteChordOverride = useCallback(
    (pasteChordOverride: PasteChordOverride) => update({ pasteChordOverride }),
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
    (macro: Macro) =>
      mutate((current) => {
        const without = current.macros.filter((item) => item.id !== macro.id);
        const clash = without.some((item) => item.trigger === macro.trigger);
        if (clash) return current;
        return { ...current, macros: [...without, macro] };
      }),
    [mutate],
  );

  const removeMacro = useCallback(
    (id: string) =>
      mutate((current) => ({
        ...current,
        macros: current.macros.filter((item) => item.id !== id),
      })),
    [mutate],
  );

  const setMacros = useCallback(
    (macros: Macro[]) => update({ macros }),
    [update],
  );

  const pushRecent = useCallback(
    (emoji: string) =>
      mutate((current) => {
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

        return {
          ...current,
          recents: nextRecents,
          usageCounts,
          firstUsedAt,
        };
      }),
    [mutate],
  );

  const clearRecents = useCallback(() => {
    update({ recents: [] });
  }, [update]);

  const clearUsageStats = useCallback(() => {
    update({ usageCounts: {}, firstUsedAt: {} });
  }, [update]);

  const toggleFavorite = useCallback(
    (hexcode: string) =>
      mutate((current) => {
        const exists = current.favorites.includes(hexcode);
        const favorites = exists
          ? current.favorites.filter((item) => item !== hexcode)
          : [hexcode, ...current.favorites];
        return { ...current, favorites };
      }),
    [mutate],
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
    setFavoriteEmojiMacros,
    setEmoticonStyle,
    setAutoPasteOnCopy,
    setExpandAsYouType,
    setExpandTriggerMode,
    setExpandKeepTriggerSpace,
    setExpandRestoreClipboard,
    setPasteChordOverride,
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
