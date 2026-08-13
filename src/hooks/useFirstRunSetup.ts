import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { InputHelperStatus } from "../lib/inputHelper";

type Options = {
  ready: boolean;
  setupSeen: boolean;
  onStatus: (status: InputHelperStatus) => void;
  onMarkSeen: () => void;
};

/** Ensures the input helper and opens first-run setup when needed. */
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

    void invoke<InputHelperStatus>("input_helper_ensure_started")
      .then((status) => {
        if (cancelled) return;
        onStatus(status);
        if (setupSeen) return;
        if (status.daemon && status.canListen) {
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
    const timer = window.setInterval(refresh, 5000);
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
