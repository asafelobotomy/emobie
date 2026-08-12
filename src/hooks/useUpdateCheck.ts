import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type UpdateCheckResult = {
  current: string;
  latest: string | null;
  newerAvailable: boolean;
  releaseUrl: string | null;
  detail: string;
};

export type TrayStatus = {
  available: boolean;
  detail: string;
};

export function useUpdateCheck(options: {
  ready: boolean;
  enabled: boolean;
  dismissedVersion: string | null;
}) {
  const [update, setUpdate] = useState<UpdateCheckResult | null>(null);

  useEffect(() => {
    if (!options.ready || !options.enabled) {
      setUpdate(null);
      return;
    }
    let cancelled = false;
    void invoke<UpdateCheckResult>("check_for_updates")
      .then((result) => {
        if (cancelled) return;
        if (
          result.newerAvailable &&
          result.latest &&
          result.latest === options.dismissedVersion
        ) {
          setUpdate({ ...result, newerAvailable: false });
          return;
        }
        setUpdate(result);
      })
      .catch(() => {
        if (!cancelled) {
          setUpdate({
            current: "",
            latest: null,
            newerAvailable: false,
            releaseUrl: null,
            detail: "Update check failed.",
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [options.ready, options.enabled, options.dismissedVersion]);

  return update;
}

export async function openReleasePage(url: string): Promise<void> {
  await invoke("open_release_page", { url });
}
