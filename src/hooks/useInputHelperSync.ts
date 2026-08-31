import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { customExpansionMatches } from "../lib/macros";
import type { InputHelperStatus } from "../lib/inputHelper";
import type { Macro, MacroTriggerMode } from "../types/preferences";

type Options = {
  ready: boolean;
  expandAsYouType: boolean;
  expandTriggerMode: MacroTriggerMode;
  expandKeepTriggerSpace: boolean;
  /** Custom user macros only — never shortcode catalog (daemon max 2000). */
  macros: Macro[];
  onStatus: (status: InputHelperStatus) => void;
  /** Called when sync/enable fails so settings can show a hard error. */
  onSyncError?: (message: string) => void;
};

function errMessage(error: unknown, fallback: string): string {
  if (typeof error === "string" && error.trim()) return error;
  if (error instanceof Error && error.message.trim()) return error.message;
  return fallback;
}

/**
 * Starts emobie-inputd on app ready, enables listening when expand-as-you-type
 * is on, and syncs *custom* macro matches (not the shortcode catalog).
 */
export function useInputHelperSync({
  ready,
  expandAsYouType,
  expandTriggerMode,
  expandKeepTriggerSpace,
  macros,
  onStatus,
  onSyncError,
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
      } catch (error) {
        if (!cancelled) {
          onSyncError?.(
            errMessage(error, "Could not start emobie-inputd."),
          );
        }
      }
      if (cancelled) return;

      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_set_enabled",
          { enabled: expandAsYouType },
        );
        if (!cancelled) onStatus(status);
      } catch (error) {
        if (!cancelled) {
          onSyncError?.(
            errMessage(
              error,
              expandAsYouType
                ? "Could not enable text expansion on the helper."
                : "Could not disable text expansion on the helper.",
            ),
          );
        }
      }
      if (cancelled || !expandAsYouType) return;

      const matches = customExpansionMatches(
        macros.map((macro) => ({ ...macro, source: "custom" as const })),
        expandTriggerMode,
        expandKeepTriggerSpace,
      );
      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_sync_matches",
          { matches },
        );
        if (!cancelled) onStatus(status);
      } catch (error) {
        if (!cancelled) {
          onSyncError?.(
            errMessage(
              error,
              "Could not sync expansion matches — Expand may be using stale macros.",
            ),
          );
        }
      }
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
    onSyncError,
  ]);
}
