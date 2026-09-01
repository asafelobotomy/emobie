import { useEffect, useMemo, useRef } from "react";
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
  const onStatusRef = useRef(onStatus);
  const onSyncErrorRef = useRef(onSyncError);
  onStatusRef.current = onStatus;
  onSyncErrorRef.current = onSyncError;

  const expansionMacrosKey = useMemo(
    () =>
      JSON.stringify(
        expansionMacros.map((m) => [
          m.trigger,
          m.expansion,
          m.enabled,
          m.source,
        ]),
      ),
    [expansionMacros],
  );
  const enableGeneration = useRef(0);
  const syncGeneration = useRef(0);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    const generation = ++enableGeneration.current;

    void (async () => {
      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_ensure_started",
        );
        if (cancelled || generation !== enableGeneration.current) return;
        onStatusRef.current(status);
      } catch (error) {
        if (cancelled || generation !== enableGeneration.current) return;
        onSyncErrorRef.current?.(
          errMessage(error, "Could not start emobie-inputd."),
        );
      }
      if (cancelled || generation !== enableGeneration.current) return;

      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_set_enabled",
          { enabled: expandAsYouType },
        );
        if (cancelled || generation !== enableGeneration.current) return;
        onStatusRef.current(status);
      } catch (error) {
        if (cancelled || generation !== enableGeneration.current) return;
        onSyncErrorRef.current?.(
          errMessage(
            error,
            expandAsYouType
              ? "Could not enable text expansion on the helper."
              : "Could not disable text expansion on the helper.",
          ),
        );
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [ready, expandAsYouType]);

  useEffect(() => {
    if (!ready || !expandAsYouType) return;
    let cancelled = false;
    const generation = ++syncGeneration.current;

    const syncToDaemon = async () => {
      if (cancelled || generation !== syncGeneration.current) return;
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
        if (cancelled || generation !== syncGeneration.current) return;
        onStatusRef.current(status);
      } catch (error) {
        if (cancelled || generation !== syncGeneration.current) return;
        onSyncErrorRef.current?.(
          errMessage(
            error,
            "Could not sync expansion matches — Expand may be using stale macros.",
          ),
        );
      }
    };

    void syncToDaemon();

    // Re-push matches if inputd restarted while emobie stayed open (tray/minimized).
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
    expansionMacrosKey,
  ]);
}
