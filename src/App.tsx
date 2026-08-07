import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Toolbar } from "./components/Toolbar";
import { CategoryNav } from "./components/CategoryNav";
import { EmojiGrid } from "./components/EmojiGrid";
import { RecentStrip } from "./components/RecentStrip";
import { SettingsPanel } from "./components/SettingsPanel";
import { WindowResizeHandles } from "./components/WindowResizeHandles";
import {
  FAVORITES_CATEGORY_ID,
  NAV_CATEGORIES,
  emojisForCategory,
  searchEmojis,
} from "./data/loadEmojis";
import { usePreferences } from "./hooks/usePreferences";
import { useLayoutMode } from "./hooks/useLayoutMode";
import { useCopyEmoji } from "./hooks/useCopyEmoji";
import { useAlwaysOnTop } from "./hooks/useAlwaysOnTop";
import { useGlobalHotkey } from "./hooks/useGlobalHotkey";
import { useTheme } from "./hooks/useTheme";
import { useWindowDecorations } from "./hooks/useWindowDecorations";
import { useAutostart } from "./hooks/useAutostart";
import { useAllowMultipleInstances } from "./hooks/useAllowMultipleInstances";
import "@fontsource/outfit/400.css";
import "@fontsource/outfit/600.css";
import "@fontsource/fraunces/600.css";
import "./styles/tokens.css";
import "./styles/app.css";
import "./styles/resize.css";
import "./styles/toolbar.css";
import "./styles/layout.css";
import "./styles/settings.css";

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
  const pinnedRef = useRef(prefs.pinned);
  pinnedRef.current = prefs.pinned;

  useTheme(prefs.theme);
  useAlwaysOnTop(prefs.pinned, ready);
  useWindowDecorations(prefs.showTitleBar, ready);
  const autostartError = useAutostart(prefs.launchOnStartup, ready);
  useAllowMultipleInstances(prefs.allowMultipleInstances, ready);
  const hotkeyError = useGlobalHotkey(
    prefs.hotkey,
    ready && !prefs.allowMultipleInstances,
  );

  useEffect(() => {
    document.documentElement.dataset.size = prefs.emojiSize;
  }, [prefs.emojiSize]);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    void invoke<boolean>("is_tray_available")
      .then((ok) => {
        if (!cancelled) setTrayUnavailable(!ok);
      })
      .catch(() => {
        if (!cancelled) setTrayUnavailable(true);
      });
    return () => {
      cancelled = true;
    };
  }, [ready]);

  const { copyEmoji, lastCopied, flashKey, copyError } = useCopyEmoji(pushRecent);

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

  const visibleEmojis = useMemo(() => {
    if (query.trim()) {
      return searchEmojis(query, prefs.skinTone, prefs.favorites, sortCtx);
    }
    return emojisForCategory(
      activeCategory,
      prefs.skinTone,
      prefs.favorites,
      sortCtx,
    );
  }, [query, activeCategory, prefs.skinTone, prefs.favorites, sortCtx]);

  const emptyMessage =
    !query.trim() && activeCategory === FAVORITES_CATEGORY_ID
      ? "Right-click an emoji to add it to Favorites."
      : "No emojis match your search.";

  const status = copyError
    ? copyError
    : hotkeyError
      ? hotkeyError
      : trayUnavailable
        ? "System tray unavailable — close quits the app."
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
          <EmojiGrid
            emojis={visibleEmojis}
            flashKey={flashKey}
            favorites={prefs.favorites}
            scrollAxis={scrollAxis}
            emptyMessage={emptyMessage}
            onCopy={copyEmoji}
            onToggleFavorite={toggleFavorite}
          />
        </div>
        <RecentStrip
          recents={prefs.recents}
          flashKey={flashKey}
          status={status}
          statusError={statusError}
          onCopy={copyEmoji}
        />
      </div>
      {settingsOpen ? (
        <SettingsPanel
          prefs={prefs}
          hotkeyError={hotkeyError}
          autostartError={autostartError}
          prefsError={prefsError}
          trayUnavailable={trayUnavailable}
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
          onClearRecents={clearRecents}
          onClearUsageStats={clearUsageStats}
        />
      ) : null}
    </div>
  );
}

export default App;
