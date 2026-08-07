import { useEffect, useId, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SkinTonePicker } from "./SkinTonePicker";
import { HotkeyCapture } from "./HotkeyCapture";
import {
  SORT_OPTIONS,
  type Preferences,
  type ThemeMode,
  type EmojiSize,
  type SortBy,
} from "../types/preferences";
import type { SkinTone } from "../data/loadEmojis";

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

type SettingsPanelProps = {
  prefs: Preferences;
  hotkeyError: string | null;
  autostartError: string | null;
  prefsError: string | null;
  trayUnavailable?: boolean;
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
  onClearRecents: () => void;
  onClearUsageStats: () => void;
};

function getFocusable(panel: HTMLElement): HTMLElement[] {
  return Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => !el.hasAttribute("disabled") && el.offsetParent !== null,
  );
}

export function SettingsPanel({
  prefs,
  hotkeyError,
  autostartError,
  prefsError,
  trayUnavailable,
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
  onClearRecents,
  onClearUsageStats,
}: SettingsPanelProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const previouslyFocused = document.activeElement as HTMLElement | null;
    const focusable = panelRef.current
      ? getFocusable(panelRef.current)
      : [];
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
        if (event.target === event.currentTarget) {
          onClose();
        }
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

        {trayUnavailable ? (
          <p className="settings-hint">
            System tray unavailable — closing the window quits the app. Use Quit
            below when you want to exit.
          </p>
        ) : null}
        {prefsError ? <p className="settings-error">{prefsError}</p> : null}

        <div className="settings-row">
          <label htmlFor="theme">Theme</label>
          <select
            id="theme"
            value={prefs.theme}
            onChange={(event) => onTheme(event.target.value as ThemeMode)}
          >
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>

        <div className="settings-row settings-toggle-row">
          <label htmlFor="show-title-bar">Show title bar</label>
          <input
            id="show-title-bar"
            type="checkbox"
            checked={prefs.showTitleBar}
            onChange={(event) => onShowTitleBar(event.target.checked)}
          />
        </div>

        <div className="settings-row settings-toggle-row">
          <label htmlFor="launch-on-startup">Launch on startup</label>
          <input
            id="launch-on-startup"
            type="checkbox"
            checked={prefs.launchOnStartup}
            onChange={(event) => onLaunchOnStartup(event.target.checked)}
          />
        </div>
        {autostartError ? (
          <p className="settings-error">{autostartError}</p>
        ) : null}

        <div className="settings-row settings-toggle-row">
          <label htmlFor="start-minimized">Start minimized to system tray</label>
          <input
            id="start-minimized"
            type="checkbox"
            checked={prefs.startMinimizedToTray}
            onChange={(event) => onStartMinimizedToTray(event.target.checked)}
          />
        </div>

        <div className="settings-row settings-toggle-row">
          <label htmlFor="allow-multiple">Allow multiple instances</label>
          <input
            id="allow-multiple"
            type="checkbox"
            checked={prefs.allowMultipleInstances}
            onChange={(event) => onAllowMultipleInstances(event.target.checked)}
          />
        </div>
        <p className="settings-hint settings-hint-block">
          Takes effect immediately. Turn off and restart emobie to enforce a
          single instance again.
        </p>

        <div className="settings-row">
          <label htmlFor="sort-by">Sort by</label>
          <select
            id="sort-by"
            value={prefs.sortBy}
            onChange={(event) => onSortBy(event.target.value as SortBy)}
          >
            {SORT_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <div className="settings-row">
          <label htmlFor="emoji-size">Emoji size</label>
          <select
            id="emoji-size"
            value={prefs.emojiSize}
            onChange={(event) => onEmojiSize(event.target.value as EmojiSize)}
          >
            <option value="sm">Small</option>
            <option value="md">Medium</option>
            <option value="lg">Large</option>
          </select>
        </div>

        <div className="settings-row">
          <span id="skin-tone-label" className="settings-label">
            Default skin tone
          </span>
          <SkinTonePicker skinTone={prefs.skinTone} onSkinTone={onSkinTone} />
        </div>

        <div className="settings-row">
          <label htmlFor="recent-max">Recent history size</label>
          <input
            id="recent-max"
            type="number"
            min={8}
            max={96}
            value={prefs.recentMax}
            onChange={(event) => {
              const value = Number(event.target.value);
              if (!Number.isFinite(value)) return;
              onRecentMax(Math.min(96, Math.max(8, Math.round(value))));
            }}
          />
        </div>

        <HotkeyCapture
          value={prefs.hotkey}
          error={hotkeyError}
          onChange={onHotkey}
        />

        <div className="settings-actions">
          <button type="button" className="btn danger" onClick={onClearRecents}>
            Clear recents
          </button>
          <button
            type="button"
            className="btn danger"
            onClick={onClearUsageStats}
          >
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
