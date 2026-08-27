import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type PinCapability = {
  wayland: boolean;
  plasma: boolean;
  reliable: boolean;
  detail: string;
};

export type PinApplyResult = {
  applied: boolean;
  limited: boolean;
  detail: string;
};

async function applyPin(pinned: boolean): Promise<PinApplyResult | null> {
  try {
    return await invoke<PinApplyResult>("apply_window_pin", { pinned });
  } catch (error) {
    console.error("Failed to apply pin via host", error);
    try {
      await getCurrentWindow().setAlwaysOnTop(pinned);
      return {
        applied: true,
        limited: false,
        detail: pinned ? "Pinned." : "Unpinned.",
      };
    } catch (fallbackError) {
      console.error("Failed to set always-on-top", fallbackError);
      return null;
    }
  }
}

/** Keep the window above others while pinned; re-apply on focus/show. */
export function useAlwaysOnTop(
  pinned: boolean,
  enabled: boolean,
  onResult?: (result: PinApplyResult | null) => void,
) {
  useEffect(() => {
    if (!enabled) return;
    const window = getCurrentWindow();
    let cancelled = false;
    const unsubs: Array<() => void> = [];

    const apply = () => {
      if (cancelled) return;
      void applyPin(pinned).then((result) => {
        if (!cancelled) onResult?.(result);
      });
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
  }, [pinned, enabled, onResult]);
}

/** One-shot compositor pin capability for Settings hints. */
export function usePinCapability(enabled: boolean) {
  const [capability, setCapability] = useState<PinCapability | null>(null);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    void invoke<PinCapability>("pin_capability")
      .then((value) => {
        if (!cancelled) setCapability(value);
      })
      .catch(() => {
        if (!cancelled) setCapability(null);
      });
    return () => {
      cancelled = true;
    };
  }, [enabled]);

  return capability;
}
