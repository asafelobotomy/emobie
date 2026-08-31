import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { InputHelperStatus } from "../lib/inputHelper";

type Options = {
  ready: boolean;
  setupSeen: boolean;
  onStatus: (status: InputHelperStatus) => void;
  onMarkSeen: () => void;
};

function expandReady(status: InputHelperStatus): boolean {
  return Boolean(status.daemon && status.canListen && status.canInject);
}

/** Ensures the input helper on every launch and opens first-run setup when needed. */
export function useFirstRunSetup({
  ready,
  setupSeen,
  onStatus,
  onMarkSeen,
}: Options) {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;

    // Always try to start emobie-inputd with the app (systemd enable --now or spawn).
    void invoke<InputHelperStatus>("input_helper_ensure_started")
      .then((status) => {
        if (cancelled) return;
        onStatus(status);
        if (setupSeen) return;
        // Match Settings: listen + inject required before treating Expand as ready.
        if (expandReady(status)) {
          onMarkSeen();
          return;
        }
        setOpen(true);
      })
      .catch(() => {
        if (!cancelled && !setupSeen) setOpen(true);
      });

    const refresh = () => {
      void invoke<InputHelperStatus>("input_helper_status")
        .then((status) => {
          if (!cancelled) onStatus(status);
        })
        .catch(() => undefined);
    };
    // Slow poll — Expand sync owns start/status on the settings path.
    const timer = window.setInterval(refresh, 15000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [ready, setupSeen, onStatus, onMarkSeen]);

  const finish = () => {
    onMarkSeen();
    setOpen(false);
  };

  return { open, finish };
}
