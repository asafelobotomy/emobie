type SettingsLifecycleHintsProps = {
  trayUnavailable?: boolean;
  trayDetail?: string | null;
  pinLimited?: boolean;
  pinDetail?: string | null;
  prefsError?: string | null;
  autostartError?: string | null;
  multiInstanceEnabled?: boolean;
};

/** Tray / pin / prefs / autostart status lines for Settings. */
export function SettingsLifecycleHints({
  trayUnavailable,
  trayDetail,
  pinLimited,
  pinDetail,
  prefsError,
  autostartError,
  multiInstanceEnabled,
}: SettingsLifecycleHintsProps) {
  return (
    <>
      {trayUnavailable ? (
        <p className="settings-hint">
          System tray unavailable — closing quits the app. GNOME: AppIndicator
          extension. Cinnamon/Mint: System Tray applet.
          {trayDetail ? ` (${trayDetail})` : ""}
        </p>
      ) : null}
      {pinLimited ? (
        <p className="settings-hint">
          Pin may be ignored on this Wayland compositor. {pinDetail}
        </p>
      ) : null}
      {prefsError ? <p className="settings-error">{prefsError}</p> : null}
      {autostartError ? (
        <p className="settings-error">{autostartError}</p>
      ) : null}
      {multiInstanceEnabled ? (
        <p className="settings-hint">
          Multiple instances share the same preference files — concurrent edits
          can overwrite each other. Use a single instance for macros and settings.
        </p>
      ) : null}
      <p className="settings-hint settings-hint-block">
        Flatpak prefers the Background portal for startup; native installs use
        XDG autostart. See docs/LINUX.md.
      </p>
    </>
  );
}
