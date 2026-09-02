import { useEffect, useId, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MacrosSettings } from "./MacrosSettings";
import { TextExpansionSettings } from "./TextExpansionSettings";
import { SettingsLifecycleHints } from "./SettingsLifecycleHints";
import { SettingsGeneralSection } from "./SettingsGeneralSection";
import { UpdateBanner } from "./UpdateBanner";
import type {
  Macro,
  MacroTriggerMode,
  EmoticonStyle,
  Preferences,
  ThemeMode,
  EmojiSize,
  SortBy,
} from "../types/preferences";
import type { SkinTone } from "../data/loadEmojis";
import { ensureInputHelperStarted } from "../lib/inputHelperClient";
import type { InputHelperStatus } from "../lib/inputHelper";
import type { PinCapability } from "../hooks/useAlwaysOnTop";
import type { UpdateCheckResult } from "../hooks/useUpdateCheck";

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

type SettingsPanelProps = {
  prefs: Preferences;
  hotkeyError: string | null;
  autostartError: string | null;
  prefsError: string | null;
  trayUnavailable?: boolean;
  trayDetail?: string | null;
  pinCapability?: PinCapability | null;
  updateInfo?: UpdateCheckResult | null;
  inputStatus: InputHelperStatus | null;
  onClose: () => void;
  onTheme: (theme: ThemeMode) => void;
  onEmojiSize: (size: EmojiSize) => void;
  onRecentMax: (value: number) => void;
  onSkinTone: (tone: SkinTone) => void;
  onHotkey: (hotkey: string) => void;
  onShowTitleBar: (show: boolean) => void;
  onLaunchOnStartup: (enabled: boolean) => void;
  onStartMinimizedToTray: (enabled: boolean) => void;
  onAllowMultipleInstances: (enabled: boolean) => void;
  onSortBy: (sortBy: SortBy) => void;
  onFavoriteEmojiMacros: (value: boolean) => void;
  onEmoticonStyle: (style: EmoticonStyle) => void;
  onAutoPasteOnCopy: (value: boolean) => void;
  onExpandAsYouType: (value: boolean) => void;
  onExpandTriggerMode: (value: MacroTriggerMode) => void;
  onExpandKeepTriggerSpace: (value: boolean) => void;
  onExpandRestoreClipboard: (value: boolean) => void;
  onHelperReconcile?: () => void;
  onCheckUpdatesOnStartup: (value: boolean) => void;
  onDismissUpdate: (version: string) => void;
  onOpenRelease: (url: string) => void;
  onSetMacros: (macros: Macro[]) => void;
  onInputStatus: (status: InputHelperStatus) => void;
  onClearRecents: () => void;
  onClearUsageStats: () => void;
};

function getFocusable(panel: HTMLElement): HTMLElement[] {
  return Array.from(
    panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
  ).filter((el) => !el.hasAttribute("disabled") && el.offsetParent !== null);
}

export function SettingsPanel({
  prefs,
  hotkeyError,
  autostartError,
  prefsError,
  trayUnavailable,
  trayDetail,
  pinCapability,
  updateInfo,
  inputStatus,
  onClose,
  onTheme,
  onEmojiSize,
  onRecentMax,
  onSkinTone,
  onHotkey,
  onShowTitleBar,
  onLaunchOnStartup,
  onStartMinimizedToTray,
  onAllowMultipleInstances,
  onSortBy,
  onFavoriteEmojiMacros,
  onEmoticonStyle,
  onAutoPasteOnCopy,
  onExpandAsYouType,
  onExpandTriggerMode,
  onExpandKeepTriggerSpace,
  onExpandRestoreClipboard,
  onHelperReconcile,
  onCheckUpdatesOnStartup,
  onDismissUpdate,
  onOpenRelease,
  onSetMacros,
  onInputStatus,
  onClearRecents,
  onClearUsageStats,
}: SettingsPanelProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const previouslyFocused = document.activeElement as HTMLElement | null;
    const focusable = panelRef.current ? getFocusable(panelRef.current) : [];
    focusable[0]?.focus();
    return () => {
      previouslyFocused?.focus?.();
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        // HotkeyCapture owns Escape while actively recording a shortcut.
        if (panelRef.current?.querySelector(".hotkey-capture.active")) {
          return;
        }
        event.preventDefault();
        event.stopPropagation();
        onClose();
        return;
      }

      if (event.key !== "Tab" || !panelRef.current) return;

      const focusable = getFocusable(panelRef.current);
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement as HTMLElement | null;

      if (event.shiftKey) {
        if (active === first || !panelRef.current.contains(active)) {
          event.preventDefault();
          last.focus();
        }
      } else if (active === last || !panelRef.current.contains(active)) {
        event.preventDefault();
        first.focus();
      }
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [onClose]);

  const quitApp = () => {
    void invoke("quit_app").catch((error) => {
      console.error("Failed to quit", error);
    });
  };

  return (
    <div
      className="settings-overlay"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <h2 id={titleId}>Preferences</h2>
        <SettingsLifecycleHints
          trayUnavailable={trayUnavailable}
          trayDetail={trayDetail}
          pinLimited={Boolean(
            prefs.pinned && pinCapability && !pinCapability.reliable,
          )}
          pinDetail={pinCapability?.detail ?? null}
          prefsError={prefsError}
          autostartError={autostartError}
          multiInstanceEnabled={prefs.allowMultipleInstances}
        />

        {updateInfo ? (
          <UpdateBanner
            updateInfo={updateInfo}
            onDismissUpdate={onDismissUpdate}
            onOpenRelease={onOpenRelease}
          />
        ) : null}

        <div className="settings-row settings-toggle-row">
          <label htmlFor="check-updates">Check for updates on startup</label>
          <input
            id="check-updates"
            type="checkbox"
            checked={prefs.checkUpdatesOnStartup}
            onChange={(event) => onCheckUpdatesOnStartup(event.target.checked)}
          />
        </div>

        <SettingsGeneralSection
          prefs={prefs}
          hotkeyError={hotkeyError}
          onTheme={onTheme}
          onEmojiSize={onEmojiSize}
          onRecentMax={onRecentMax}
          onSkinTone={onSkinTone}
          onHotkey={onHotkey}
          onShowTitleBar={onShowTitleBar}
          onLaunchOnStartup={onLaunchOnStartup}
          onStartMinimizedToTray={onStartMinimizedToTray}
          onAllowMultipleInstances={onAllowMultipleInstances}
          onSortBy={onSortBy}
          onEmoticonStyle={onEmoticonStyle}
        />

        <h3 className="settings-section-title">Clipboard</h3>
        <div className="settings-row settings-toggle-row">
          <label htmlFor="auto-paste">Auto-paste on copy</label>
          <input
            id="auto-paste"
            type="checkbox"
            checked={prefs.autoPasteOnCopy}
            onChange={(event) => {
              const enabled = event.target.checked;
              onAutoPasteOnCopy(enabled);
              if (enabled) {
                void ensureInputHelperStarted()
                  .then(onInputStatus)
                  .catch(() => undefined);
              }
            }}
          />
        </div>
        <p className="settings-hint settings-hint-block">
          When on, emobie hides after copy and pastes into the previous app
          (Ctrl+V). Turn this off to only copy to the clipboard. Skipped while
          pinned or if the system tray is unavailable.
        </p>

        <TextExpansionSettings
          expandAsYouType={prefs.expandAsYouType}
          expandTriggerMode={prefs.expandTriggerMode}
          expandKeepTriggerSpace={prefs.expandKeepTriggerSpace}
          expandRestoreClipboard={prefs.expandRestoreClipboard}
          inputStatus={inputStatus}
          onExpandAsYouType={onExpandAsYouType}
          onExpandTriggerMode={onExpandTriggerMode}
          onExpandKeepTriggerSpace={onExpandKeepTriggerSpace}
          onExpandRestoreClipboard={onExpandRestoreClipboard}
          onInputStatus={onInputStatus}
          onHelperReconcile={onHelperReconcile}
        />

        <MacrosSettings
          macros={prefs.macros}
          favorites={prefs.favorites}
          favoriteEmojiMacros={prefs.favoriteEmojiMacros}
          onFavoriteEmojiMacros={onFavoriteEmojiMacros}
          onSetMacros={onSetMacros}
        />

        <div className="settings-actions">
          <button type="button" className="btn danger" onClick={onClearRecents}>
            Clear recents
          </button>
          <button type="button" className="btn danger" onClick={onClearUsageStats}>
            Reset usage stats
          </button>
          <button type="button" className="btn" onClick={quitApp}>
            Quit
          </button>
          <button type="button" className="btn primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
