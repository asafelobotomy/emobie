import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Apply OS title-bar decorations from the showTitleBar preference. */
export function useWindowDecorations(showTitleBar: boolean, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    const window = getCurrentWindow();
    void window.setDecorations(showTitleBar).catch((error) => {
      console.error("Failed to update window decorations", error);
    });
  }, [showTitleBar, enabled]);
}
