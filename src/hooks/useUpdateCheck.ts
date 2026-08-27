import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type InstallKind =
  | "flatpak"
  | "appImage"
  | "deb"
  | "rpm"
  | "native";

export type UpdateCheckResult = {
  current: string;
  latest: string | null;
  newerAvailable: boolean;
  releaseUrl: string | null;
  detail: string;
  downloadUrl: string | null;
  assetName: string | null;
  installKind: InstallKind;
  canAutoUpdate: boolean;
};

export type ApplyUpdateResult = {
  ok: boolean;
  detail: string;
  restartRequired: boolean;
};

export type TrayStatus = {
  available: boolean;
  detail: string;
};

const FAILED_CHECK: UpdateCheckResult = {
  current: "",
  latest: null,
  newerAvailable: false,
  releaseUrl: null,
  detail: "Update check failed.",
  downloadUrl: null,
  assetName: null,
  installKind: "native",
  canAutoUpdate: false,
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
        if (!cancelled) setUpdate(FAILED_CHECK);
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

export async function applyUpdate(
  downloadUrl: string,
  assetName: string,
): Promise<ApplyUpdateResult> {
  return invoke<ApplyUpdateResult>("apply_update", {
    downloadUrl,
    assetName,
  });
}
