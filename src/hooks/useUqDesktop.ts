import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Whether uQ is the running desktop (org.uq.Desktop1 has an owner on the
 * session bus); null while the check runs. Checked once per mount.
 */
export function useUqDesktop(): boolean | null {
  const [active, setActive] = useState<boolean | null>(null);
  useEffect(() => {
    let cancelled = false;
    void invoke<boolean>("uq_desktop_active")
      .then((value) => {
        if (!cancelled) setActive(value === true);
      })
      .catch(() => {
        if (!cancelled) setActive(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);
  return active;
}
