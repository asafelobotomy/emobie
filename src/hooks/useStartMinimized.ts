import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Hide to tray once on startup when requested — but only if the tray is
 * available, otherwise the window would vanish with no way to restore it.
 */
export function useStartMinimized(startMinimized: boolean, ready: boolean) {
  const appliedRef = useRef(false);

  useEffect(() => {
    if (!ready || appliedRef.current) return;
    appliedRef.current = true;
    if (!startMinimized) return;

    void (async () => {
      try {
        const trayOk = await invoke<boolean>("is_tray_available");
        if (!trayOk) {
          console.warn("Skipping start minimized: system tray unavailable");
          return;
        }
        await getCurrentWindow().hide();
      } catch (error) {
        console.error("Failed to start minimized", error);
      }
    })();
  }, [ready, startMinimized]);
}
