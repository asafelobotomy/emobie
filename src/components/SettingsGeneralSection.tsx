import type { Preferences } from "../types/preferences";
import type { SkinTone } from "../data/loadEmojis";
import type {
  EmoticonStyle,
  EmojiSize,
  SortBy,
  ThemeMode,
} from "../types/preferences";
import {
  SORT_OPTIONS,
  EMOTICON_STYLE_OPTIONS,
} from "../types/preferences";
import { SkinTonePicker } from "./SkinTonePicker";
import { HotkeyCapture } from "./HotkeyCapture";

type SettingsGeneralSectionProps = {
  prefs: Preferences;
  hotkeyError: string | null;
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
  onEmoticonStyle: (style: EmoticonStyle) => void;
};

export function SettingsGeneralSection({
  prefs,
  hotkeyError,
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
  onEmoticonStyle,
}: SettingsGeneralSectionProps) {
  return (
    <>
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
        single instance again. With multiple instances enabled, preference
        writes from different windows can race.
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
        <label htmlFor="emoticon-style">Emoticon style</label>
        <select
          id="emoticon-style"
          value={prefs.emoticonStyle}
          onChange={(event) =>
            onEmoticonStyle(event.target.value as EmoticonStyle)
          }
        >
          {EMOTICON_STYLE_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
      <p className="settings-hint settings-hint-block">
        Controls which ASCII emoticon triggers are used for favorite emoji
        macros — for example <code>:)</code> vs <code>:-)</code>.
      </p>

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
    </>
  );
}
