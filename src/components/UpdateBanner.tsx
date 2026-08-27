import { useState } from "react";
import type { UpdateCheckResult } from "../hooks/useUpdateCheck";
import { applyUpdate } from "../hooks/useUpdateCheck";

type UpdateBannerProps = {
  updateInfo: UpdateCheckResult;
  onDismissUpdate: (version: string) => void;
  onOpenRelease: (url: string) => void;
};

export function UpdateBanner({
  updateInfo,
  onDismissUpdate,
  onOpenRelease,
}: UpdateBannerProps) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!updateInfo.newerAvailable) return null;

  const runUpdate = () => {
    if (!updateInfo.downloadUrl || !updateInfo.assetName || busy) return;
    setBusy(true);
    setError(null);
    setMessage("Downloading and installing…");
    void applyUpdate(updateInfo.downloadUrl, updateInfo.assetName)
      .then((result) => {
        setMessage(result.detail);
        setBusy(false);
      })
      .catch((err: unknown) => {
        setMessage(null);
        setError(err instanceof Error ? err.message : String(err));
        setBusy(false);
      });
  };

  return (
    <div className="settings-update-banner">
      <p className="settings-hint">
        {updateInfo.detail} (you have v{updateInfo.current})
      </p>
      {message ? <p className="settings-hint">{message}</p> : null}
      {error ? <p className="settings-error">{error}</p> : null}
      <div className="settings-actions">
        {updateInfo.canAutoUpdate && updateInfo.downloadUrl ? (
          <button
            type="button"
            className="btn primary"
            disabled={busy}
            onClick={runUpdate}
          >
            {busy ? "Updating…" : "Update now"}
          </button>
        ) : null}
        {updateInfo.releaseUrl ? (
          <button
            type="button"
            className={
              updateInfo.canAutoUpdate ? "btn" : "btn primary"
            }
            disabled={busy}
            onClick={() => onOpenRelease(updateInfo.releaseUrl!)}
          >
            Open release
          </button>
        ) : null}
        {updateInfo.latest ? (
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => onDismissUpdate(updateInfo.latest!)}
          >
            Dismiss
          </button>
        ) : null}
      </div>
    </div>
  );
}
