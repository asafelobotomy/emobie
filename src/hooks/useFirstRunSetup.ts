import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { InputHelperStatus } from "../lib/inputHelper";

type Options = {
  ready: boolean;
  setupSeen: boolean;
  onStatus: (status: InputHelperStatus) => void;
  onMarkSeen: () => void;
};

// As-you-type text expansion is deferred (see docs/MACROS.md "Known
// limitations"), so first-run only needs paste-injection access, not
// keyboard-listen access (canListen is never granted).
function pasteReady(status: InputHelperStatus): boolean {
  return Boolean(
    status.daemon && status.canInject && status.accessConfigured !== false,
  );
}

/** Opens first-run setup when needed; status polling only (start/sync owns ensure). */
export function useFirstRunSetup({
  ready,
  setupSeen,
  onStatus,
  onMarkSeen,
}: Options) {
  const [open, setOpen] = useState(false);
  const onStatusRef = useRef(onStatus);
  const onMarkSeenRef = useRef(onMarkSeen);
  onStatusRef.current = onStatus;
  onMarkSeenRef.current = onMarkSeen;

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;

    const refresh = () => {
      void invoke<InputHelperStatus>("input_helper_status")
        .then((status) => {
          if (cancelled) return;
          onStatusRef.current(status);
          if (setupSeen) return;
          if (pasteReady(status)) {
            onMarkSeenRef.current();
            return;
          }
          setOpen(true);
        })
        .catch(() => {
          if (!cancelled && !setupSeen) setOpen(true);
        });
    };

    refresh();
    const timer = window.setInterval(refresh, 15000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [ready, setupSeen]);

  const finish = () => {
    onMarkSeenRef.current();
    setOpen(false);
  };

  return { open, finish };
}
