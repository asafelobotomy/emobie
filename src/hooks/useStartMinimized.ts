import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Hide to tray once on startup when the preference is enabled. */
export function useStartMinimized(startMinimized: boolean, ready: boolean) {
  const appliedRef = useRef(false);

  useEffect(() => {
    if (!ready || appliedRef.current) return;
    appliedRef.current = true;
    if (!startMinimized) return;

    void getCurrentWindow()
      .hide()
      .catch((error) => {
        console.error("Failed to start minimized", error);
      });
  }, [ready, startMinimized]);
}
