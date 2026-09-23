import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type PinCapability = {
  wayland: boolean;
  plasma: boolean;
  reliable: boolean;
  detail: string;
  /** True on GNOME Wayland when the toggle-above shortcut isn't set up yet. */
  gnomeSetupNeeded?: boolean;
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

/**
 * Keep the window above others while pinned; re-apply on focus/show.
 * Returns why the last pin attempt failed, or null when it applied.
 */
export function useAlwaysOnTop(pinned: boolean, enabled: boolean) {
  const [notice, setNotice] = useState<string | null>(null);
  const onResult = useCallback((result: PinApplyResult | null) => {
    if (!result) setNotice("Could not pin the window.");
    else setNotice(result.applied ? null : result.detail);
  }, []);

  useEffect(() => {
    if (!enabled) return;
    const window = getCurrentWindow();
    let cancelled = false;
    const unsubs: Array<() => void> = [];

    const apply = () => {
      if (cancelled) return;
      void applyPin(pinned).then((result) => {
        if (!cancelled) onResult(result);
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

    return () => {
      cancelled = true;
      for (const unsub of unsubs) unsub();
    };
  }, [pinned, enabled, onResult]);

  return notice;
}

/** Compositor pin capability for Settings hints, with a manual refresh. */
export function usePinCapability(enabled: boolean) {
  const [capability, setCapability] = useState<PinCapability | null>(null);

  const refresh = useCallback(() => {
    return invoke<PinCapability>("pin_capability")
      .then((value) => {
        setCapability(value);
        return value;
      })
      .catch(() => {
        setCapability(null);
        return null;
      });
  }, []);

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

  return { capability, refresh };
}
