import { useCallback, useEffect, useState } from "react";
import { SkinTonePicker } from "./SkinTonePicker";
import type { Preferences, ThemeMode, EmojiSize } from "../types/preferences";
import type { SkinTone } from "../data/loadEmojis";

type SettingsPanelProps = {
  prefs: Preferences;
  onClose: () => void;
  onTheme: (theme: ThemeMode) => void;
  onEmojiSize: (size: EmojiSize) => void;
  onRecentMax: (value: number) => void;
  onSkinTone: (tone: SkinTone) => void;
  onHotkey: (hotkey: string) => void;
  onClearRecents: () => void;
};

function formatHotkey(event: KeyboardEvent): string | null {
  if (["Shift", "Control", "Alt", "Meta"].includes(event.key)) {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  if (event.metaKey) parts.push("Meta");

  let key = event.key;
  if (key === " ") key = "Space";
  if (key.length === 1) key = key.toUpperCase();

  parts.push(key);
  return parts.join("+");
}

export function SettingsPanel({
  prefs,
  onClose,
  onTheme,
  onEmojiSize,
  onRecentMax,
  onSkinTone,
  onHotkey,
  onClearRecents,
}: SettingsPanelProps) {
  const [capturing, setCapturing] = useState(false);
  const [draftHotkey, setDraftHotkey] = useState(prefs.hotkey);

  useEffect(() => {
    setDraftHotkey(prefs.hotkey);
  }, [prefs.hotkey]);

  useEffect(() => {
    if (!capturing) return;

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setCapturing(false);
        return;
      }
      const next = formatHotkey(event);
      if (!next) return;
      setDraftHotkey(next);
      onHotkey(next);
      setCapturing(false);
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capturing, onHotkey]);

  const startCapture = useCallback(() => {
    setCapturing(true);
  }, []);

  return (
    <div className="settings-overlay" role="dialog" aria-modal="true" aria-label="Settings">
      <div className="settings-panel">
        <h2>Preferences</h2>

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
          <span id="skin-tone-label">Default skin tone</span>
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
          <label>Global hotkey</label>
          <button
            type="button"
            className={`hotkey-capture${capturing ? " active" : ""}`}
            onClick={startCapture}
          >
            {capturing ? "Press a shortcut…" : draftHotkey}
          </button>
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
