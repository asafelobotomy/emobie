import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

async function applyPin(pinned: boolean) {
  try {
    await invoke("apply_window_pin", { pinned });
  } catch (error) {
    // Fallback when the command is unavailable (older build).
    console.error("Failed to apply pin via host", error);
    try {
      await getCurrentWindow().setAlwaysOnTop(pinned);
    } catch (fallbackError) {
      console.error("Failed to set always-on-top", fallbackError);
    }
  }
}

/** Keep the window above others while pinned; re-apply on focus/show. */
export function useAlwaysOnTop(pinned: boolean, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    const window = getCurrentWindow();
    let cancelled = false;
    const unsubs: Array<() => void> = [];

    const apply = () => {
      if (!cancelled) void applyPin(pinned);
    };

    apply();

    void window
      .onFocusChanged(({ payload: focused }) => {
        if (focused && pinned) apply();
      })
      .then((unsub) => {
        if (cancelled) unsub();
        else unsubs.push(unsub);
      });

    void window
      .listen("tauri://focus", () => {
        if (pinned) apply();
      })
      .then((unsub) => {
        if (cancelled) unsub();
        else unsubs.push(unsub);
      });

    return () => {
      cancelled = true;
      for (const unsub of unsubs) unsub();
    };
  }, [pinned, enabled]);
}
