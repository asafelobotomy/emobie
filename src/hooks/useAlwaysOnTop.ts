import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useAlwaysOnTop(pinned: boolean, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    void getCurrentWindow().setAlwaysOnTop(pinned);
  }, [pinned, enabled]);
}
