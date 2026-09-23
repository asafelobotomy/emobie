import { useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorMessage } from "../lib/errorMessage";
import { customExpansionMatches, type MacroEntry } from "../lib/macroHelpers";
import type { InputHelperStatus } from "../lib/inputHelper";
import type { Preferences } from "../types/preferences";

export type HelperSyncPrefs = Pick<
  Preferences,
  | "expandAsYouType"
  | "expandTriggerMode"
  | "expandKeepTriggerSpace"
  | "expandRestoreClipboard"
  | "expandExcludedApps"
  | "pasteChordOverride"
>;

type Options = {
  ready: boolean;
  prefs: HelperSyncPrefs;
  /** Custom macros plus optional favorited-emoji macros. */
  expansionMacros: MacroEntry[];
  /** Bump after Grant restarts the daemon to re-apply state (in-memory state is lost). */
  reconcileNonce?: number;
  onStatus: (status: InputHelperStatus) => void;
  /** Called when sync/enable fails so settings can show a hard error. */
  onSyncError?: (message: string) => void;
};

/** Serialize helper IPC so cancelled effect runs cannot reorder enable/sync. */
let helperSyncChain: Promise<void> = Promise.resolve();

function enqueueHelperSync(work: () => Promise<void>): Promise<void> {
  const run = helperSyncChain.then(work, work);
  // Keep the chain alive even when work rejects.
  helperSyncChain = run.catch(() => {});
  return run;
}

/**
 * Starts emobie-inputd, applies options and — when Expand as you type is on —
 * syncs matches then enables listening. Enabling always goes
 * disable → sync → enable so a daemon already enabled at login can never
 * expand stale rules.
 */
export function useInputHelperSync({
  ready,
  prefs,
  expansionMacros,
  reconcileNonce = 0,
  onStatus,
  onSyncError,
}: Options) {
  const onStatusRef = useRef(onStatus);
  const onSyncErrorRef = useRef(onSyncError);
  onStatusRef.current = onStatus;
  onSyncErrorRef.current = onSyncError;

  const {
    expandAsYouType,
    expandTriggerMode,
    expandKeepTriggerSpace,
    expandRestoreClipboard,
    expandExcludedApps,
    pasteChordOverride,
  } = prefs;

  const matches = useMemo(
    () =>
      customExpansionMatches(
        expansionMacros,
        expandTriggerMode,
        expandKeepTriggerSpace,
      ),
    [expansionMacros, expandTriggerMode, expandKeepTriggerSpace],
  );
  const matchesKey = JSON.stringify(matches);
  const excludedKey = JSON.stringify(expandExcludedApps);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    const isCurrent = () => !cancelled;

    const pushStatus = (status: InputHelperStatus) => {
      if (isCurrent()) onStatusRef.current(status);
    };
    const fail = (error: unknown, fallback: string) => {
      if (isCurrent()) onSyncErrorRef.current?.(errorMessage(error, fallback));
    };
    const call = async (cmd: string, args?: Record<string, unknown>) => {
      const status = await invoke<InputHelperStatus>(cmd, args);
      pushStatus(status);
      return status;
    };

    void enqueueHelperSync(async () => {
      if (!isCurrent()) return;
      let started: InputHelperStatus;
      try {
        started = await call("input_helper_ensure_started");
      } catch (error) {
        fail(error, "Could not start emobie-inputd.");
        return;
      }
      if (!isCurrent()) return;

      try {
        await call("input_helper_set_options", {
          restoreClipboard: expandRestoreClipboard,
          pasteChord: pasteChordOverride,
          excludeApps: JSON.parse(excludedKey) as string[],
        });
      } catch (error) {
        // Options are best-effort; older helpers may lack set_options.
        if (isCurrent()) console.warn("input_helper_set_options failed", error);
      }
      if (!isCurrent()) return;

      try {
        await call("input_helper_set_enabled", { enabled: false });
      } catch (error) {
        fail(error, "Could not pause text expansion on the helper.");
        return;
      }
      if (!expandAsYouType || !isCurrent()) return;
      // The opt-in read rule is the consent record: a preference saved by an
      // older release must not start keyboard reading on its own.
      if (!started.keyboardReadConfigured) {
        fail(null, "Expand as you type needs keyboard access — turn it off and on in Settings to grant it.");
        return;
      }

      try {
        await call("input_helper_sync_matches", {
          matches: JSON.parse(matchesKey),
        });
      } catch (error) {
        fail(error, "Could not send your macros to the helper — Expand stays off.");
        return;
      }
      if (!isCurrent()) return;

      try {
        await call("input_helper_set_enabled", { enabled: true });
      } catch (error) {
        fail(error, "Could not turn on text expansion.");
      }
    });

    return () => {
      cancelled = true;
    };
  }, [
    ready,
    expandAsYouType,
    expandRestoreClipboard,
    pasteChordOverride,
    excludedKey,
    matchesKey,
    reconcileNonce,
  ]);
}
