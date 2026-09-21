import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorMessage } from "../lib/errorMessage";

type SettingsLifecycleHintsProps = {
  trayUnavailable?: boolean;
  trayDetail?: string | null;
  pinLimited?: boolean;
  pinDetail?: string | null;
  /** True on GNOME Wayland when the toggle-above shortcut isn't set up yet. */
  pinGnomeSetupNeeded?: boolean;
  onRefreshPinCapability?: () => void;
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
  pinGnomeSetupNeeded,
  onRefreshPinCapability,
  prefsError,
  autostartError,
  multiInstanceEnabled,
}: SettingsLifecycleHintsProps) {
  const [busy, setBusy] = useState(false);
  const [setupMessage, setSetupMessage] = useState<string | null>(null);

  const setupGnomePin = async () => {
    setBusy(true);
    setSetupMessage(null);
    try {
      await invoke("pin_gnome_setup");
      onRefreshPinCapability?.();
      setSetupMessage("Pin shortcut set up — try Pin again.");
    } catch (error) {
      setSetupMessage(errorMessage(error, "Could not set up pin shortcut."));
    } finally {
      setBusy(false);
    }
  };

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
      {pinGnomeSetupNeeded ? (
        <div className="settings-actions macros-io-actions">
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => void setupGnomePin()}
          >
            {busy ? "Working…" : "Set up GNOME pin shortcut"}
          </button>
          <p className="settings-hint settings-hint-block">
            One-time setup: binds GNOME's unused "toggle above" shortcut
            (Ctrl+Alt+Super+F12) so emobie can trigger it. Won't touch any
            shortcut you've already set.
          </p>
        </div>
      ) : null}
      {setupMessage ? <p className="settings-hint">{setupMessage}</p> : null}
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
