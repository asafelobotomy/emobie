import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Toolbar } from "./components/Toolbar";
import { CategoryNav } from "./components/CategoryNav";
import { EmojiGrid } from "./components/EmojiGrid";
import { RecentStrip } from "./components/RecentStrip";
import { SettingsPanel } from "./components/SettingsPanel";
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
import "@fontsource/outfit/400.css";
import "@fontsource/outfit/600.css";
import "@fontsource/fraunces/600.css";
import "./styles/tokens.css";
import "./styles/app.css";

function App() {
  const {
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
  } = usePreferences();

  const [rootEl, setRootEl] = useState<HTMLDivElement | null>(null);
  const { mode: layout, scrollAxis, compact } = useLayoutMode(rootEl);
  const [query, setQuery] = useState("");
  const [activeCategory, setActiveCategory] = useState(FAVORITES_CATEGORY_ID);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const pinnedRef = useRef(prefs.pinned);
  pinnedRef.current = prefs.pinned;

  useTheme(prefs.theme);
  useAlwaysOnTop(prefs.pinned, ready);
  const hotkeyError = useGlobalHotkey(prefs.hotkey, ready);

  useEffect(() => {
    document.documentElement.dataset.size = prefs.emojiSize;
  }, [prefs.emojiSize]);

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

  const visibleEmojis = useMemo(() => {
    if (query.trim()) {
      return searchEmojis(query, prefs.skinTone, prefs.favorites);
    }
    return emojisForCategory(activeCategory, prefs.skinTone, prefs.favorites);
  }, [query, activeCategory, prefs.skinTone, prefs.favorites]);

  const emptyMessage =
    !query.trim() && activeCategory === FAVORITES_CATEGORY_ID
      ? "Right-click an emoji to add it to Favorites."
      : "No emojis match your search.";

  const status = copyError
    ? copyError
    : lastCopied
      ? `Copied ${lastCopied}`
      : null;

  return (
    <div className="app-shell" ref={setRootEl}>
      <div
        className="app"
        data-layout={layout}
        data-scroll={scrollAxis}
        data-compact={compact ? "true" : "false"}
      >
        <Toolbar
          query={query}
          onQueryChange={setQuery}
          pinned={prefs.pinned}
          onTogglePin={togglePin}
          onOpenSettings={() => setSettingsOpen(true)}
          compact={compact}
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
          statusError={Boolean(copyError)}
          onCopy={copyEmoji}
        />
      </div>
      {settingsOpen ? (
        <SettingsPanel
          prefs={prefs}
          hotkeyError={hotkeyError}
          onClose={() => setSettingsOpen(false)}
          onTheme={setTheme}
          onEmojiSize={setEmojiSize}
          onRecentMax={setRecentMax}
          onSkinTone={setSkinTone}
          onHotkey={setHotkey}
          onClearRecents={clearRecents}
        />
      ) : null}
    </div>
  );
}

export default App;
