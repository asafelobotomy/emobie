import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Keep desktop autostart registration in sync with the preference. */
export function useAutostart(launchOnStartup: boolean, enabled: boolean) {
  const syncedRef = useRef(false);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;

    void (async () => {
      try {
        const currentlyEnabled = await invoke<boolean>("is_launch_on_startup");
        if (cancelled) return;

        if (launchOnStartup !== currentlyEnabled) {
          await invoke("set_launch_on_startup", { enabled: launchOnStartup });
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
