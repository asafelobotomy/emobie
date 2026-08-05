import { useEffect, useRef } from "react";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";

/** Keep desktop autostart registration in sync with the preference. */
export function useAutostart(launchOnStartup: boolean, enabled: boolean) {
  const syncedRef = useRef(false);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;

    void (async () => {
      try {
        const currentlyEnabled = await isEnabled();
        if (cancelled) return;

        if (launchOnStartup && !currentlyEnabled) {
          await enable();
        } else if (!launchOnStartup && currentlyEnabled) {
          await disable();
        }
        syncedRef.current = true;
      } catch (error) {
        console.error("Failed to update autostart", error);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [launchOnStartup, enabled]);
}
