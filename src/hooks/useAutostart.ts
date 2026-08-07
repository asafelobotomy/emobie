import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Keep desktop autostart registration in sync with the preference. */
export function useAutostart(launchOnStartup: boolean, enabled: boolean) {
  const syncedRef = useRef(false);
  const [error, setError] = useState<string | null>(null);

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
        if (!cancelled) setError(null);
      } catch (err) {
        console.error("Failed to update autostart", err);
        if (!cancelled) {
          setError("Could not update launch on startup.");
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [launchOnStartup, enabled]);

  return error;
}
