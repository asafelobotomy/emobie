import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useAlwaysOnTop(pinned: boolean) {
  useEffect(() => {
    void getCurrentWindow().setAlwaysOnTop(pinned);
  }, [pinned]);
}
