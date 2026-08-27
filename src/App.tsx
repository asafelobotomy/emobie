import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Toolbar } from "./components/Toolbar";
import { CategoryNav } from "./components/CategoryNav";
import { EmojiGrid } from "./components/EmojiGrid";
import { MacroList } from "./components/MacroList";
import { RecentStrip } from "./components/RecentStrip";
import { FirstRunSetup } from "./components/FirstRunSetup";
import { SettingsPanel } from "./components/SettingsPanel";
import { WindowResizeHandles } from "./components/WindowResizeHandles";
import {
  FAVORITES_CATEGORY_ID,
  MACROS_CATEGORY_ID,
  NAV_CATEGORIES,
  emojisForCategory,
  searchEmojis,
} from "./data/loadEmojis";
import { mergeMacros, searchMacros } from "./lib/macros";
import type { InputHelperStatus } from "./lib/inputHelper";
import { usePreferences } from "./hooks/usePreferences";
import { useLayoutMode } from "./hooks/useLayoutMode";
import { useCopyEmoji, useCopyText } from "./hooks/useCopyEmoji";
import { useAlwaysOnTop } from "./hooks/useAlwaysOnTop";
import { useGlobalHotkeys } from "./hooks/useGlobalHotkey";
import { useTheme } from "./hooks/useTheme";
import { useWindowDecorations } from "./hooks/useWindowDecorations";
import { useAutostart } from "./hooks/useAutostart";
import { useAllowMultipleInstances } from "./hooks/useAllowMultipleInstances";
import { useFirstRunSetup } from "./hooks/useFirstRunSetup";
import { useInputHelperSync } from "./hooks/useInputHelperSync";
import {
  openReleasePage,
  useUpdateCheck,
  type TrayStatus,
} from "./hooks/useUpdateCheck";
import "@fontsource/outfit/400.css";
import "@fontsource/outfit/600.css";
import "@fontsource/fraunces/600.css";
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
    setShowShortcodeMacros,
    setAutoPasteOnCopy,
    setExpandAsYouType,
    setExpandTriggerMode,
    setExpandKeepTriggerSpace,
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
  const markSetupSeen = useCallback(() => {
    setInputHelperSetupSeen(true);
  }, [setInputHelperSetupSeen]);
  const { open: firstRunOpen, finish: finishFirstRun } = useFirstRunSetup({
    ready,
    setupSeen: prefs.inputHelperSetupSeen,
    onStatus: setInputStatus,
    onMarkSeen: markSetupSeen,
  });
  const pinnedRef = useRef(prefs.pinned);
  pinnedRef.current = prefs.pinned;

  useTheme(prefs.theme);
  useAlwaysOnTop(prefs.pinned, ready);
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
        showShortcodes: prefs.showShortcodeMacros,
        skinTone: prefs.skinTone,
      }),
    [prefs.macros, prefs.showShortcodeMacros, prefs.skinTone],
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
    macros: mergedMacros,
    onStatus: setInputStatus,
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
    ? prefs.macros.length === 0 && !prefs.showShortcodeMacros
      ? "Tap + to add a macro, or enable emoji shortcodes in Settings."
      : "No macros match your search."
    : !query.trim() && activeCategory === FAVORITES_CATEGORY_ID
      ? "Right-click an emoji to add it to Favorites."
      : "No emojis match your search.";

  const status = copyError
    ? copyError
    : hotkeyError
      ? hotkeyError
      : trayUnavailable
        ? "System tray unavailable — close quits the app."
        : updateInfo?.newerAvailable
          ? updateInfo.detail
          : lastCopied
            ? `Copied ${lastCopied}`
            : null;

  const statusError = Boolean(copyError || hotkeyError || trayUnavailable);
  const frameless = !prefs.showTitleBar;

  return (
    <div className="app-shell" ref={setRootEl}>
      <WindowResizeHandles enabled={frameless && !settingsOpen} />
      <div
        className="app"
        data-layout={layout}
        data-scroll={scrollAxis}
        data-compact={compact ? "true" : "false"}
        data-frameless={frameless ? "true" : "false"}
      >
        <Toolbar
          query={query}
          onQueryChange={setQuery}
          pinned={prefs.pinned}
          onTogglePin={togglePin}
          onOpenSettings={() => setSettingsOpen(true)}
          frameless={frameless}
          trayUnavailable={trayUnavailable}
        />
        <div className="body">
          <CategoryNav
            categories={NAV_CATEGORIES}
            activeId={activeCategory}
            onSelect={(id) => {
              setActiveCategory(id);
              setQuery("");
            }}
          />
          {macrosMode ? (
            <MacroList
              macros={visibleMacros}
              customMacros={prefs.macros}
              summonHotkey={prefs.hotkey}
              flashKey={flashKey}
              emptyMessage={emptyMessage}
              searchActive={query.trim().length > 0}
              onCopy={copyMacro}
              onUpsert={upsertMacro}
              onRemove={removeMacro}
            />
          ) : (
            <EmojiGrid
              emojis={visibleEmojis}
              flashKey={flashKey}
              favorites={prefs.favorites}
              scrollAxis={scrollAxis}
              emptyMessage={emptyMessage}
              onCopy={copyEmojiWithPaste}
              onToggleFavorite={toggleFavorite}
            />
          )}
        </div>
        <RecentStrip
          recents={prefs.recents}
          flashKey={flashKey}
          status={status}
          statusError={statusError}
          onCopy={copyEmojiWithPaste}
        />
      </div>
      {settingsOpen ? (
        <SettingsPanel
          prefs={prefs}
          hotkeyError={hotkeyError}
          autostartError={autostartError}
          prefsError={prefsError}
          trayUnavailable={trayUnavailable}
          trayDetail={trayDetail}
          updateInfo={updateInfo}
          inputStatus={inputStatus}
          onClose={() => setSettingsOpen(false)}
          onTheme={setTheme}
          onEmojiSize={setEmojiSize}
          onRecentMax={setRecentMax}
          onSkinTone={setSkinTone}
          onHotkey={setHotkey}
          onShowTitleBar={setShowTitleBar}
          onLaunchOnStartup={setLaunchOnStartup}
          onStartMinimizedToTray={setStartMinimizedToTray}
          onAllowMultipleInstances={setAllowMultipleInstances}
          onSortBy={setSortBy}
          onShowShortcodeMacros={setShowShortcodeMacros}
          onAutoPasteOnCopy={setAutoPasteOnCopy}
          onExpandAsYouType={setExpandAsYouType}
          onExpandTriggerMode={setExpandTriggerMode}
          onExpandKeepTriggerSpace={setExpandKeepTriggerSpace}
          onCheckUpdatesOnStartup={setCheckUpdatesOnStartup}
          onDismissUpdate={(version) => setDismissedUpdateVersion(version)}
          onOpenRelease={(url) => {
            void openReleasePage(url).catch((error) => {
              console.error("Failed to open release page", error);
            });
          }}
          onSetMacros={setMacros}
          onInputStatus={setInputStatus}
          onClearRecents={clearRecents}
          onClearUsageStats={clearUsageStats}
        />
      ) : null}
      <FirstRunSetup
        open={firstRunOpen}
        status={inputStatus}
        onStatus={setInputStatus}
        onDone={finishFirstRun}
      />
    </div>
  );
}

export default App;
