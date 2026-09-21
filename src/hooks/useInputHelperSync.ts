import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorMessage } from "../lib/errorMessage";
import type { InputHelperStatus } from "../lib/inputHelper";
import type { PasteChordOverride } from "../types/preferences";

type Options = {
  ready: boolean;
  restoreClipboard: boolean;
  pasteChord: PasteChordOverride;
  /** Bump after Grant restarts the daemon to re-apply options (in-memory state is lost). */
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
 * Starts emobie-inputd on app ready and applies paste-related options.
 *
 * As-you-type text expansion (trigger listening + macro sync) is deferred
 * for now — see docs/MACROS.md "Known limitations". This hook only keeps
 * the daemon available for the "Auto-paste on copy" preference, and makes
 * sure listening stays off regardless of any stale saved preference from a
 * version where expansion was enabled.
 */
export function useInputHelperSync({
  ready,
  restoreClipboard,
  pasteChord,
  reconcileNonce = 0,
  onStatus,
  onSyncError,
}: Options) {
  const onStatusRef = useRef(onStatus);
  const onSyncErrorRef = useRef(onSyncError);
  onStatusRef.current = onStatus;
  onSyncErrorRef.current = onSyncError;

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;

    const isCurrent = () => !cancelled;

    const pushStatus = (status: InputHelperStatus) => {
      if (!isCurrent()) return;
      onStatusRef.current(status);
    };

    const fail = (error: unknown, fallback: string) => {
      if (!isCurrent()) return;
      onSyncErrorRef.current?.(errorMessage(error, fallback));
    };

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
          { restoreClipboard, pasteChord },
        );
        pushStatus(status);
      } catch (error) {
        // Options are best-effort; older helpers may lack set_options.
        if (isCurrent()) {
          console.warn("input_helper_set_options failed", error);
        }
      }
      if (!isCurrent()) return;

      // Text expansion is deferred — always keep listening off, even if a
      // preference file saved from an older version still says otherwise.
      try {
        const status = await invoke<InputHelperStatus>(
          "input_helper_set_enabled",
          { enabled: false },
        );
        pushStatus(status);
      } catch (error) {
        fail(error, "Could not disable text expansion on the helper.");
      }
    });

    return () => {
      cancelled = true;
    };
  }, [ready, restoreClipboard, pasteChord, reconcileNonce]);
}
