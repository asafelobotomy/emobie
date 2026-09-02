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
  expandRestoreClipboard: boolean;
  /** Bump after Grant/restart to force disable→sync→enable once. */
  reconcileNonce?: number;
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

/** Serialize helper IPC so cancelled effect runs cannot reorder enable/sync. */
let helperSyncChain: Promise<void> = Promise.resolve();

function enqueueHelperSync(work: () => Promise<void>): Promise<void> {
  const run = helperSyncChain.then(work, work);
  // Keep the chain alive even when work rejects.
  helperSyncChain = run.catch(() => {});
  return run;
}

/**
 * Starts emobie-inputd on app ready, enables listening when expand-as-you-type
 * is on, and syncs custom + favorited-emoji macro matches.
 *
 * When enabling: disable → sync matches → enable, so a daemon that was already
 * enabled at login cannot expand stale rules, and in-flight toggles cannot
 * reorder across overlapping effect runs.
 */
export function useInputHelperSync({
  ready,
  expandAsYouType,
  expandTriggerMode,
  expandKeepTriggerSpace,
  expandRestoreClipboard,
  reconcileNonce = 0,
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

    const isCurrent = () =>
      !cancelled && generation === syncGeneration.current;

    const pushStatus = (status: InputHelperStatus) => {
      if (!isCurrent()) return;
      onStatusRef.current(status);
    };

    const fail = (error: unknown, fallback: string) => {
      if (!isCurrent()) return;
      onSyncErrorRef.current?.(errMessage(error, fallback));
    };

    const matches = () =>
      customExpansionMatches(
        expansionMacros,
        expandTriggerMode,
        expandKeepTriggerSpace,
      );

    void enqueueHelperSync(async () => {
      if (!isCurrent()) return;

      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_ensure_started",
        );
        pushStatus(status);
      } catch (error) {
        fail(error, "Could not start emobie-inputd.");
        return;
      }
      if (!isCurrent()) return;

      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_set_options",
          { restoreClipboard: expandRestoreClipboard },
        );
        pushStatus(status);
      } catch (error) {
        // Options are best-effort; older helpers may lack set_options.
        if (isCurrent()) {
          console.warn("input_helper_set_options failed", error);
        }
      }
      if (!isCurrent()) return;

      if (expandAsYouType) {
        // Pause matching before swapping the trie when the daemon may already
        // be enabled from a prior session / login unit.
        try {
          const status = await invoke<InputHelperStatus>(
            "input_helper_set_enabled",
            { enabled: false },
          );
          pushStatus(status);
        } catch (error) {
          fail(error, "Could not pause text expansion before syncing macros.");
          return;
        }
        if (!isCurrent()) return;

        try {
          const status = await invoke<InputHelperStatus>(
            "input_helper_sync_matches",
            { matches: matches() },
          );
          pushStatus(status);
        } catch (error) {
          fail(
            error,
            "Could not sync expansion matches — Expand may be using stale macros.",
          );
          return;
        }
        if (!isCurrent()) return;

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
    });

    // Re-push matches if inputd restarted while emobie stayed open (tray/minimized).
    // Daemon no-ops identical syncs without rewriting disk.
    let resync: ReturnType<typeof setInterval> | undefined;
    if (expandAsYouType) {
      resync = setInterval(() => {
        if (!isCurrent()) return;
        void enqueueHelperSync(async () => {
          if (!isCurrent()) return;
          try {
            const status = await invoke<InputHelperStatus>(
              "input_helper_sync_matches",
              { matches: matches() },
            );
            pushStatus(status);
          } catch (error) {
            fail(
              error,
              "Could not sync expansion matches — Expand may be using stale macros.",
            );
          }
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
    expandRestoreClipboard,
    reconcileNonce,
    expansionMacrosKey,
  ]);
}
