import { useState } from "react";
import type { InputHelperStatus } from "../lib/inputHelper";
import { runInputHelperAccessSetup } from "../lib/inputHelperClient";

type PasteAccessSettingsProps = {
  autoPasteOnCopy: boolean;
  restoreClipboard: boolean;
  inputStatus: InputHelperStatus | null;
  onRestoreClipboard: (value: boolean) => void;
  onInputStatus: (status: InputHelperStatus) => void;
  onHelperReconcile?: () => void;
};

function helperStatusLabel(status: InputHelperStatus | null): string {
  if (!status) return "Checking paste helper…";
  if (status.daemon && status.canInject && status.accessConfigured === false) {
    return `Paste helper ready for now, but permanent access is incomplete (group/udev). Use Grant to repair. ${status.detail}`;
  }
  if (status.daemon && status.canInject) {
    return `Paste helper running. ${status.detail}`;
  }
  if (status.daemon && !status.canInject) {
    return `Paste helper running, but text injection is unavailable (need writable /dev/uinput on Wayland). Use Grant to repair. ${status.detail}`;
  }
  return status.detail;
}

/**
 * Grant/repair UI for the paste-injection half of emobie-inputd, used by
 * "Auto-paste on copy". As-you-type text expansion (the listen half) is
 * deferred for now — see docs/MACROS.md "Known limitations".
 */
export function PasteAccessSettings({
  autoPasteOnCopy,
  restoreClipboard,
  inputStatus,
  onRestoreClipboard,
  onInputStatus,
  onHelperReconcile,
}: PasteAccessSettingsProps) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const canInject = Boolean(inputStatus?.canInject);
  const accessConfigured = inputStatus?.accessConfigured !== false;
  const daemonReady = Boolean(inputStatus?.daemon);
  const needsGrant = daemonReady && (!canInject || !accessConfigured);

  if (!autoPasteOnCopy) return null;

  const retryGrant = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const status = await runInputHelperAccessSetup();
      onInputStatus(status);
      setMessage(status.detail);
      onHelperReconcile?.();
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "Paste access setup failed.",
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="text-expansion-settings">
      <p className="settings-hint settings-hint-block">
        {helperStatusLabel(inputStatus)}
      </p>

      {needsGrant ? (
        <div className="settings-actions macros-io-actions">
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => void retryGrant()}
          >
            {busy
              ? "Working…"
              : canInject && !accessConfigured
                ? "Repair paste access"
                : "Grant paste access"}
          </button>
        </div>
      ) : null}

      <div className="settings-row settings-toggle-row">
        <label htmlFor="expand-restore-clipboard">
          Restore clipboard after paste
        </label>
        <input
          id="expand-restore-clipboard"
          type="checkbox"
          checked={restoreClipboard}
          onChange={(event) => onRestoreClipboard(event.target.checked)}
        />
      </div>
      <p className="settings-hint settings-hint-block">
        Off by default (recommended on Plasma Wayland). Optional{" "}
        <code>wl-clipboard</code> improves paste reliability.
      </p>

      {message ? <p className="settings-hint">{message}</p> : null}
    </div>
  );
}
