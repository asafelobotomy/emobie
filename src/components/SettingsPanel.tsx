import { useCallback, useEffect, useId, useRef, useState } from "react";
import { SkinTonePicker } from "./SkinTonePicker";
import {
  SORT_OPTIONS,
  type Preferences,
  type ThemeMode,
  type EmojiSize,
  type SortBy,
} from "../types/preferences";
import type { SkinTone } from "../data/loadEmojis";

type SettingsPanelProps = {
  prefs: Preferences;
  hotkeyError: string | null;
  onClose: () => void;
  onTheme: (theme: ThemeMode) => void;
  onEmojiSize: (size: EmojiSize) => void;
  onRecentMax: (value: number) => void;
  onSkinTone: (tone: SkinTone) => void;
  onHotkey: (hotkey: string) => void;
  onShowTitleBar: (show: boolean) => void;
  onLaunchOnStartup: (enabled: boolean) => void;
  onStartMinimizedToTray: (enabled: boolean) => void;
  onSortBy: (sortBy: SortBy) => void;
  onClearRecents: () => void;
};

function isLetterOrDigitKey(key: string): boolean {
  return key.length === 1 && /[A-Za-z0-9]/.test(key);
}

function formatHotkey(event: KeyboardEvent): string | null {
  if (["Shift", "Control", "Alt", "Meta"].includes(event.key)) {
    return null;
  }

  const hasActionModifier =
    event.ctrlKey || event.altKey || event.metaKey || event.shiftKey;

  // Bare letters/digits are reserved for typing; everything else is allowed.
  if (isLetterOrDigitKey(event.key) && !hasActionModifier) {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  if (event.metaKey) parts.push("Meta");

  let key = event.key;
  if (key === " ") key = "Space";
  else if (key === "ArrowUp") key = "Up";
  else if (key === "ArrowDown") key = "Down";
  else if (key === "ArrowLeft") key = "Left";
  else if (key === "ArrowRight") key = "Right";
  else if (key === "Escape") key = "Esc";
  else if (key.length === 1) key = key.toUpperCase();

  parts.push(key);
  return parts.join("+");
}

export function SettingsPanel({
  prefs,
  hotkeyError,
  onClose,
  onTheme,
  onEmojiSize,
  onRecentMax,
  onSkinTone,
  onHotkey,
  onShowTitleBar,
  onLaunchOnStartup,
  onStartMinimizedToTray,
  onSortBy,
  onClearRecents,
}: SettingsPanelProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const [capturing, setCapturing] = useState(false);
  const [draftHotkey, setDraftHotkey] = useState(prefs.hotkey);
  const [hotkeyHint, setHotkeyHint] = useState<string | null>(null);

  useEffect(() => {
    setDraftHotkey(prefs.hotkey);
  }, [prefs.hotkey]);

  useEffect(() => {
    const previouslyFocused = document.activeElement as HTMLElement | null;
    const focusable = panelRef.current?.querySelector<HTMLElement>(
      "select, button, input",
    );
    focusable?.focus();

    return () => {
      previouslyFocused?.focus?.();
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      if (capturing) {
        setCapturing(false);
        setHotkeyHint(null);
        return;
      }
      onClose();
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capturing, onClose]);

  useEffect(() => {
    if (!capturing) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      const next = formatHotkey(event);
      if (!next) {
        setHotkeyHint("Letter and number keys need Ctrl, Alt, Shift, or Meta");
        return;
      }
      setDraftHotkey(next);
      onHotkey(next);
      setHotkeyHint(null);
      setCapturing(false);
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capturing, onHotkey]);

  const startCapture = useCallback(() => {
    setHotkeyHint(null);
    setCapturing(true);
  }, []);

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

        <div className="settings-row settings-toggle-row">
          <label htmlFor="start-minimized">Start minimized to system tray</label>
          <input
            id="start-minimized"
            type="checkbox"
            checked={prefs.startMinimizedToTray}
            onChange={(event) => onStartMinimizedToTray(event.target.checked)}
          />
        </div>

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

        <div className="settings-row">
          <span className="settings-label">Global hotkey</span>
          <button
            type="button"
            className={`hotkey-capture${capturing ? " active" : ""}`}
            onClick={startCapture}
          >
            {capturing ? "Press a shortcut…" : draftHotkey}
          </button>
          {hotkeyHint ? <p className="settings-hint">{hotkeyHint}</p> : (
            <p className="settings-hint">
              Letters and numbers need Ctrl, Alt, Shift, or Meta. Function keys and
              punctuation can stand alone.
            </p>
          )}
          {hotkeyError ? <p className="settings-error">{hotkeyError}</p> : null}
        </div>

        <div className="settings-actions">
          <button type="button" className="btn danger" onClick={onClearRecents}>
            Clear recents
          </button>
          <button type="button" className="btn primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
