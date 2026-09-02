import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AppShell } from "./components/AppShell";
import {
  FAVORITES_CATEGORY_ID,
  MACROS_CATEGORY_ID,
  emojisForCategory,
  searchEmojis,
} from "./data/loadEmojis";
import { mergeMacros, searchMacros } from "./lib/macros";
import type { InputHelperStatus } from "./lib/inputHelper";
import { usePreferences } from "./hooks/usePreferences";
import { useLayoutMode } from "./hooks/useLayoutMode";
import { useCopyEmoji, useCopyText } from "./hooks/useCopyEmoji";
import { useAlwaysOnTop, usePinCapability } from "./hooks/useAlwaysOnTop";
import { useGlobalHotkeys } from "./hooks/useGlobalHotkey";
import { useTheme } from "./hooks/useTheme";
import { useWindowDecorations } from "./hooks/useWindowDecorations";
import { useAutostart } from "./hooks/useAutostart";
import { useAllowMultipleInstances } from "./hooks/useAllowMultipleInstances";
import { useFirstRunSetup } from "./hooks/useFirstRunSetup";
import { useInputHelperSync } from "./hooks/useInputHelperSync";
import {
  useUpdateCheck,
  type TrayStatus,
} from "./hooks/useUpdateCheck";
import "@fontsource/ubuntu/400.css";
import "@fontsource/ubuntu/500.css";
import "@fontsource/ubuntu/700.css";
import "@fontsource/ubuntu-mono/400.css";
import "@fontsource/ubuntu-mono/700.css";
import "./styles/tokens.css";
import "./styles/app.css";
import "./styles/resize.css";
import "./styles/toolbar.css";
import "./styles/layout.css";
import "./styles/settings.css";
import "./styles/macros.css";

function App() {
  const {
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
  } = usePreferences();

  const [rootEl, setRootEl] = useState<HTMLDivElement | null>(null);
  const { mode: layout, scrollAxis, compact } = useLayoutMode(rootEl);
  const [query, setQuery] = useState("");
  const [activeCategory, setActiveCategory] = useState(FAVORITES_CATEGORY_ID);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [trayUnavailable, setTrayUnavailable] = useState(false);
  const [trayDetail, setTrayDetail] = useState<string | null>(null);
  const [inputStatus, setInputStatus] = useState<InputHelperStatus | null>(
    null,
  );
  const [inputError, setInputError] = useState<string | null>(null);
  const [reconcileNonce, setReconcileNonce] = useState(0);
  const bumpHelperReconcile = useCallback(() => {
    setReconcileNonce((n) => n + 1);
  }, []);
  const markSetupSeen = useCallback(() => {
    setInputHelperSetupSeen(true);
  }, [setInputHelperSetupSeen]);

  const handleInputStatus = useCallback((status: InputHelperStatus) => {
    setInputStatus(status);
    setInputError(null);
  }, []);

  const handleInputSyncError = useCallback((message: string) => {
    setInputError(message);
    setInputStatus((prev) =>
      prev
        ? { ...prev, detail: message }
        : {
            daemon: false,
            canInject: false,
            canListen: false,
            detail: message,
            accessConfigured: false,
          },
    );
  }, []);
  const { open: firstRunOpen, finish: finishFirstRun } = useFirstRunSetup({
    ready,
    setupSeen: prefs.inputHelperSetupSeen,
    onStatus: handleInputStatus,
    onMarkSeen: markSetupSeen,
  });
  const pinnedRef = useRef(prefs.pinned);
  pinnedRef.current = prefs.pinned;

  useTheme(prefs.theme);
  useAlwaysOnTop(prefs.pinned, ready);
  const pinCapability = usePinCapability(ready);
  useWindowDecorations(prefs.showTitleBar, ready);
  const autostartError = useAutostart(prefs.launchOnStartup, ready);
  useAllowMultipleInstances(prefs.allowMultipleInstances, ready);

  const {
    copyEmoji,
    lastCopied: emojiCopied,
    flashKey: emojiFlash,
    copyError: emojiCopyError,
  } = useCopyEmoji(pushRecent);
  const {
    copyText,
    lastCopied: textCopied,
    flashKey: textFlash,
    copyError: textCopyError,
  } = useCopyText();

  const lastCopied = textCopied ?? emojiCopied;
  const flashKey = textFlash ?? emojiFlash;
  const copyError = textCopyError ?? emojiCopyError;

  // Auto-paste hides the palette; skip when pinned or tray is missing.
  const autoPasteOpts = useMemo(() => {
    if (!prefs.autoPasteOnCopy || prefs.pinned || trayUnavailable) {
      return { autoPaste: false as const };
    }
    return { autoPaste: true as const, hideForPaste: true as const };
  }, [prefs.autoPasteOnCopy, prefs.pinned, trayUnavailable]);

  const copyMacro = useCallback(
    (text: string, flashKey?: string) => {
      void copyText(text, { ...autoPasteOpts, flashKey });
    },
    [copyText, autoPasteOpts],
  );

  const copyEmojiWithPaste = useCallback(
    (emoji: string) => {
      void copyEmoji(emoji, autoPasteOpts);
    },
    [copyEmoji, autoPasteOpts],
  );

  const hotkeyError = useGlobalHotkeys({
    summonHotkey: prefs.hotkey,
    summonEnabled: ready && !prefs.allowMultipleInstances,
    pinned: prefs.pinned,
    macros: prefs.macros,
    onMacroHotkey: copyMacro,
    ready,
  });

  const mergedMacros = useMemo(
    () =>
      mergeMacros(prefs.macros, {
        favoriteEmojiMacros: prefs.favoriteEmojiMacros,
        favorites: prefs.favorites,
        skinTone: prefs.skinTone,
        emoticonStyle: prefs.emoticonStyle,
      }),
    [
      prefs.macros,
      prefs.favoriteEmojiMacros,
      prefs.favorites,
      prefs.skinTone,
      prefs.emoticonStyle,
    ],
  );

  const visibleMacros = useMemo(() => {
    if (activeCategory !== MACROS_CATEGORY_ID && !query.trim()) {
      return [];
    }
    if (query.trim() && activeCategory === MACROS_CATEGORY_ID) {
      return searchMacros(mergedMacros, query);
    }
    if (activeCategory === MACROS_CATEGORY_ID) {
      return searchMacros(mergedMacros, query);
    }
    return [];
  }, [activeCategory, mergedMacros, query]);

  const updateInfo = useUpdateCheck({
    ready,
    enabled: prefs.checkUpdatesOnStartup,
    dismissedVersion: prefs.dismissedUpdateVersion,
  });

  useEffect(() => {
    document.documentElement.dataset.size = prefs.emojiSize;
  }, [prefs.emojiSize]);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    void invoke<TrayStatus>("tray_status")
      .then((status) => {
        if (cancelled) return;
        setTrayUnavailable(!status.available);
        setTrayDetail(status.detail);
      })
      .catch(() => {
        if (!cancelled) {
          setTrayUnavailable(true);
          setTrayDetail("Tray status unavailable.");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [ready]);

  useInputHelperSync({
    ready,
    expandAsYouType: prefs.expandAsYouType,
    expandTriggerMode: prefs.expandTriggerMode,
    expandKeepTriggerSpace: prefs.expandKeepTriggerSpace,
    expandRestoreClipboard: prefs.expandRestoreClipboard,
    reconcileNonce,
    expansionMacros: mergedMacros,
    onStatus: handleInputStatus,
    onSyncError: handleInputSyncError,
  });

  const togglePin = useCallback(() => {
    setPinned(!pinnedRef.current);
  }, [setPinned]);

  useEffect(() => {
    let unlistenPin: (() => void) | undefined;
    void listen("tray-pin-toggle", () => {
      setPinned(!pinnedRef.current);
    }).then((fn) => {
      unlistenPin = fn;
    });
    return () => {
      unlistenPin?.();
    };
  }, [setPinned]);

  const sortCtx = useMemo(
    () => ({
      sortBy: prefs.sortBy,
      usageCounts: prefs.usageCounts,
      firstUsedAt: prefs.firstUsedAt,
    }),
    [prefs.sortBy, prefs.usageCounts, prefs.firstUsedAt],
  );

  const macrosMode = activeCategory === MACROS_CATEGORY_ID;

  const visibleEmojis = useMemo(() => {
    if (macrosMode) return [];
    if (query.trim()) {
      return searchEmojis(query, prefs.skinTone, prefs.favorites, sortCtx);
    }
    return emojisForCategory(
      activeCategory,
      prefs.skinTone,
      prefs.favorites,
      sortCtx,
    );
  }, [query, activeCategory, prefs.skinTone, prefs.favorites, sortCtx, macrosMode]);

  const emptyMessage = macrosMode
    ? prefs.macros.length === 0 &&
      !(prefs.favoriteEmojiMacros && prefs.favorites.length > 0)
      ? "Tap + to add a macro, or enable favorited emoji macros in Settings."
      : "No macros match your search."
    : !query.trim() && activeCategory === FAVORITES_CATEGORY_ID
      ? "Right-click an emoji to add it to Favorites."
      : "No emojis match your search.";

  const status = copyError
    ? copyError
    : hotkeyError
      ? hotkeyError
      : inputError
        ? inputError
        : trayUnavailable
          ? "System tray unavailable — close quits the app."
          : updateInfo?.newerAvailable
            ? updateInfo.detail
            : lastCopied
              ? `Copied ${lastCopied}`
              : null;

  const statusError = Boolean(
    copyError || hotkeyError || inputError || trayUnavailable,
  );
  const frameless = !prefs.showTitleBar;

  return (
    <AppShell
      rootRef={setRootEl}
      layout={layout}
      scrollAxis={scrollAxis}
      compact={compact}
      frameless={frameless}
      settingsOpen={settingsOpen}
      query={query}
      setQuery={setQuery}
      prefs={prefs}
      activeCategory={activeCategory}
      setActiveCategory={setActiveCategory}
      macrosMode={macrosMode}
      visibleMacros={visibleMacros}
      visibleEmojis={visibleEmojis}
      emptyMessage={emptyMessage}
      flashKey={flashKey ?? null}
      status={status}
      statusError={statusError}
      trayUnavailable={trayUnavailable}
      trayDetail={trayDetail}
      hotkeyError={hotkeyError}
      autostartError={autostartError}
      prefsError={prefsError}
      pinCapability={pinCapability}
      updateInfo={updateInfo}
      inputStatus={inputStatus}
      firstRunOpen={firstRunOpen}
      onTogglePin={togglePin}
      onOpenSettings={() => setSettingsOpen(true)}
      onCloseSettings={() => setSettingsOpen(false)}
      onCopyMacro={copyMacro}
      onCopyEmoji={copyEmojiWithPaste}
      onToggleFavorite={toggleFavorite}
      upsertMacro={upsertMacro}
      removeMacro={removeMacro}
      setTheme={setTheme}
      setEmojiSize={setEmojiSize}
      setRecentMax={setRecentMax}
      setSkinTone={setSkinTone}
      setHotkey={setHotkey}
      setShowTitleBar={setShowTitleBar}
      setLaunchOnStartup={setLaunchOnStartup}
      setStartMinimizedToTray={setStartMinimizedToTray}
      setAllowMultipleInstances={setAllowMultipleInstances}
      setSortBy={setSortBy}
      setFavoriteEmojiMacros={setFavoriteEmojiMacros}
      setEmoticonStyle={setEmoticonStyle}
      setAutoPasteOnCopy={setAutoPasteOnCopy}
      setExpandAsYouType={setExpandAsYouType}
      setExpandTriggerMode={setExpandTriggerMode}
      setExpandKeepTriggerSpace={setExpandKeepTriggerSpace}
      setExpandRestoreClipboard={setExpandRestoreClipboard}
      onHelperReconcile={bumpHelperReconcile}
      setCheckUpdatesOnStartup={setCheckUpdatesOnStartup}
      setDismissedUpdateVersion={setDismissedUpdateVersion}
      setMacros={setMacros}
      handleInputStatus={handleInputStatus}
      clearRecents={clearRecents}
      clearUsageStats={clearUsageStats}
      finishFirstRun={finishFirstRun}
    />
  );
}

export default App;
