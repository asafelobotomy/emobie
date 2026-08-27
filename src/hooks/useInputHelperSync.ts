import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { expansionMatches } from "../lib/macros";
import type { MacroEntry } from "../lib/macros";
import type { InputHelperStatus } from "../lib/inputHelper";
import type { MacroTriggerMode } from "../types/preferences";

type Options = {
  ready: boolean;
  expandAsYouType: boolean;
  expandTriggerMode: MacroTriggerMode;
  expandKeepTriggerSpace: boolean;
  macros: MacroEntry[];
  onStatus: (status: InputHelperStatus) => void;
};

/**
 * Always starts emobie-inputd on app ready, then enables listening when
 * expand-as-you-type is on and syncs macro matches.
 */
export function useInputHelperSync({
  ready,
  expandAsYouType,
  expandTriggerMode,
  expandKeepTriggerSpace,
  macros,
  onStatus,
}: Options) {
  useEffect(() => {
    if (!ready) return;
    let cancelled = false;

    void (async () => {
      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_ensure_started",
        );
        if (!cancelled) onStatus(status);
      } catch {
        // Helper may be missing; expand stays off until install/setup.
      }
      if (cancelled) return;

      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_set_enabled",
          { enabled: expandAsYouType },
        );
        if (!cancelled) onStatus(status);
      } catch {
        // Ignore — UI still reflects the preference.
      }
      if (cancelled || !expandAsYouType) return;

      const matches = expansionMatches(
        macros,
        expandTriggerMode,
        expandKeepTriggerSpace,
      );
      void invoke("input_helper_sync_matches", { matches }).catch(
        () => undefined,
      );
    })();

    return () => {
      cancelled = true;
    };
  }, [
    ready,
    expandAsYouType,
    expandTriggerMode,
    expandKeepTriggerSpace,
    macros,
    onStatus,
  ]);
}
