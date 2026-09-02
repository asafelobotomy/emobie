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
 *
 * When enabling, matches are synced *before* set_enabled so the first keystrokes
 * cannot expand against a stale on-disk rule set.
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
  const syncGeneration = useRef(0);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    const generation = ++syncGeneration.current;

    const pushStatus = (status: InputHelperStatus) => {
      if (cancelled || generation !== syncGeneration.current) return;
      onStatusRef.current(status);
    };

    const fail = (error: unknown, fallback: string) => {
      if (cancelled || generation !== syncGeneration.current) return;
      onSyncErrorRef.current?.(errMessage(error, fallback));
    };

    const matches = () =>
      customExpansionMatches(
        expansionMacros,
        expandTriggerMode,
        expandKeepTriggerSpace,
      );

    const syncMatches = async () => {
      const status = await invoke<InputHelperStatus>(
        "input_helper_sync_matches",
        { matches: matches() },
      );
      pushStatus(status);
      return status;
    };

    void (async () => {
      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_ensure_started",
        );
        pushStatus(status);
      } catch (error) {
        fail(error, "Could not start emobie-inputd.");
        return;
      }
      if (cancelled || generation !== syncGeneration.current) return;

      if (expandAsYouType) {
        try {
          await syncMatches();
        } catch (error) {
          fail(
            error,
            "Could not sync expansion matches — Expand may be using stale macros.",
          );
          return;
        }
        if (cancelled || generation !== syncGeneration.current) return;
        try {
          const status = await invoke<InputHelperStatus>(
            "input_helper_set_enabled",
            { enabled: true },
          );
          pushStatus(status);
        } catch (error) {
          fail(error, "Could not enable text expansion on the helper.");
        }
      } else {
        try {
          const status = await invoke<InputHelperStatus>(
            "input_helper_set_enabled",
            { enabled: false },
          );
          pushStatus(status);
        } catch (error) {
          fail(error, "Could not disable text expansion on the helper.");
        }
      }
    })();

    // Re-push matches if inputd restarted while emobie stayed open (tray/minimized).
    // Daemon no-ops identical syncs without rewriting disk.
    let resync: ReturnType<typeof setInterval> | undefined;
    if (expandAsYouType) {
      resync = setInterval(() => {
        if (cancelled || generation !== syncGeneration.current) return;
        void syncMatches().catch((error) => {
          fail(
            error,
            "Could not sync expansion matches — Expand may be using stale macros.",
          );
        });
      }, 20_000);
    }

    return () => {
      cancelled = true;
      if (resync) clearInterval(resync);
    };
  }, [
    ready,
    expandAsYouType,
    expandTriggerMode,
    expandKeepTriggerSpace,
    expansionMacrosKey,
  ]);
}
