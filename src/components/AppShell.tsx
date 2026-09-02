import type { Macro, Preferences } from "../types/preferences";
import type { MacroEntry } from "../lib/macros";
import type { InputHelperStatus } from "../lib/inputHelper";
import type { PinCapability } from "../hooks/useAlwaysOnTop";
import type { UpdateCheckResult } from "../hooks/useUpdateCheck";
import type { LayoutMode, ScrollAxis } from "../hooks/useLayoutMode";
import type { emobieEmoji, SkinTone } from "../data/loadEmojis";
import { NAV_CATEGORIES } from "../data/loadEmojis";
import { openReleasePage } from "../hooks/useUpdateCheck";
import type {
  EmoticonStyle,
  EmojiSize,
  MacroTriggerMode,
  SortBy,
  ThemeMode,
} from "../types/preferences";
import { Toolbar } from "./Toolbar";
import { CategoryNav } from "./CategoryNav";
import { EmojiGrid } from "./EmojiGrid";
import { MacroList } from "./MacroList";
import { RecentStrip } from "./RecentStrip";
import { FirstRunSetup } from "./FirstRunSetup";
import { SettingsPanel } from "./SettingsPanel";
import { WindowResizeHandles } from "./WindowResizeHandles";

export type AppShellProps = {
  rootRef: (el: HTMLDivElement | null) => void;
  layout: LayoutMode;
  scrollAxis: ScrollAxis;
  compact: boolean;
  frameless: boolean;
  settingsOpen: boolean;
  query: string;
  setQuery: (q: string) => void;
  prefs: Preferences;
  activeCategory: number;
  setActiveCategory: (id: number) => void;
  macrosMode: boolean;
  visibleMacros: MacroEntry[];
  visibleEmojis: emobieEmoji[];
  emptyMessage: string;
  flashKey: string | null;
  status: string | null;
  statusError: boolean;
  trayUnavailable: boolean;
  trayDetail: string | null;
  hotkeyError: string | null;
  autostartError: string | null;
  prefsError: string | null;
  pinCapability: PinCapability | null;
  updateInfo: UpdateCheckResult | null | undefined;
  inputStatus: InputHelperStatus | null;
  firstRunOpen: boolean;
  onTogglePin: () => void;
  onOpenSettings: () => void;
  onCloseSettings: () => void;
  onCopyMacro: (text: string, flashKey?: string) => void;
  onCopyEmoji: (emoji: string) => void;
  onToggleFavorite: (emoji: string) => void;
  upsertMacro: (macro: Macro) => void;
  removeMacro: (id: string) => void;
  setTheme: (theme: ThemeMode) => void;
  setEmojiSize: (size: EmojiSize) => void;
  setRecentMax: (value: number) => void;
  setSkinTone: (tone: SkinTone) => void;
  setHotkey: (hotkey: string) => void;
  setShowTitleBar: (show: boolean) => void;
  setLaunchOnStartup: (enabled: boolean) => void;
  setStartMinimizedToTray: (enabled: boolean) => void;
  setAllowMultipleInstances: (enabled: boolean) => void;
  setSortBy: (sortBy: SortBy) => void;
  setFavoriteEmojiMacros: (value: boolean) => void;
  setEmoticonStyle: (style: EmoticonStyle) => void;
  setAutoPasteOnCopy: (value: boolean) => void;
  setExpandAsYouType: (value: boolean) => void;
  setExpandTriggerMode: (value: MacroTriggerMode) => void;
  setExpandKeepTriggerSpace: (value: boolean) => void;
  setCheckUpdatesOnStartup: (value: boolean) => void;
  setDismissedUpdateVersion: (version: string) => void;
  setMacros: (macros: Macro[]) => void;
  handleInputStatus: (status: InputHelperStatus) => void;
  clearRecents: () => void;
  clearUsageStats: () => void;
  finishFirstRun: () => void;
};

export function AppShell(props: AppShellProps) {
  const {
    rootRef,
    layout,
    scrollAxis,
    compact,
    frameless,
    settingsOpen,
    query,
    setQuery,
    prefs,
    activeCategory,
    setActiveCategory,
    macrosMode,
    visibleMacros,
    visibleEmojis,
    emptyMessage,
    flashKey,
    status,
    statusError,
    trayUnavailable,
    trayDetail,
    hotkeyError,
    autostartError,
    prefsError,
    pinCapability,
    updateInfo,
    inputStatus,
    firstRunOpen,
    onTogglePin,
    onOpenSettings,
    onCloseSettings,
    onCopyMacro,
    onCopyEmoji,
    onToggleFavorite,
    upsertMacro,
    removeMacro,
    setTheme,
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
    setCheckUpdatesOnStartup,
    setDismissedUpdateVersion,
    setMacros,
    handleInputStatus,
    clearRecents,
    clearUsageStats,
    finishFirstRun,
  } = props;

  return (
    <div className="app-shell" ref={rootRef}>
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
          onTogglePin={onTogglePin}
          onOpenSettings={onOpenSettings}
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
              onCopy={onCopyMacro}
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
              onCopy={onCopyEmoji}
              onToggleFavorite={onToggleFavorite}
            />
          )}
        </div>
        <RecentStrip
          recents={prefs.recents}
          flashKey={flashKey}
          status={status}
          statusError={statusError}
          onCopy={onCopyEmoji}
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
          pinCapability={pinCapability}
          updateInfo={updateInfo}
          inputStatus={inputStatus}
          onClose={onCloseSettings}
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
          onFavoriteEmojiMacros={setFavoriteEmojiMacros}
          onEmoticonStyle={setEmoticonStyle}
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
          onInputStatus={handleInputStatus}
          onClearRecents={clearRecents}
          onClearUsageStats={clearUsageStats}
        />
      ) : null}
      <FirstRunSetup
        open={firstRunOpen}
        status={inputStatus}
        onStatus={handleInputStatus}
        onDone={finishFirstRun}
      />
    </div>
  );
}
