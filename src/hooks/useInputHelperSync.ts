import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { customExpansionMatches, type MacroEntry } from "../lib/macros";
import type { InputHelperStatus } from "../lib/inputHelper";
import type { MacroTriggerMode } from "../types/preferences";

type Options = {
  ready: boolean;
  expandAsYouType: boolean;
  expandTriggerMode: MacroTriggerMode;
  expandKeepTriggerSpace: boolean;
  /** Custom macros plus optional favorited-emoji macros (subset, not full catalog). */
  expansionMacros: MacroEntry[];
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
 * is on, and syncs custom + favorited-emoji macro matches.
 */
export function useInputHelperSync({
  ready,
  expandAsYouType,
  expandTriggerMode,
  expandKeepTriggerSpace,
  expansionMacros,
  onStatus,
  onSyncError,
}: Options) {
  useEffect(() => {
    if (!ready) return;
    let cancelled = false;

    const syncToDaemon = async () => {
      if (cancelled || !expandAsYouType) return;
      const matches = customExpansionMatches(
        expansionMacros,
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
    };

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
      await syncToDaemon();
    })();

    // Re-push matches if inputd restarted while emobie stayed open (tray/minimized).
    if (!expandAsYouType) return;
    const resync = setInterval(() => {
      void syncToDaemon();
    }, 20_000);

    return () => {
      cancelled = true;
      clearInterval(resync);
    };
  }, [
    ready,
    expandAsYouType,
    expandTriggerMode,
    expandKeepTriggerSpace,
    expansionMacros,
    onStatus,
    onSyncError,
  ]);
}
