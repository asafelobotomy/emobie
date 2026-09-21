import { useEffect, useId, useState } from "react";
import type { InputHelperStatus } from "../lib/inputHelper";
import { prepareInputHelperForPaste } from "../lib/inputHelperClient";
import { errorMessage } from "../lib/errorMessage";

type FirstRunSetupProps = {
  open: boolean;
  status: InputHelperStatus | null;
  onStatus: (status: InputHelperStatus) => void;
  onDone: () => void;
};

const setupError = (error: unknown) =>
  errorMessage(error, "Setup failed or was cancelled.");

export function FirstRunSetup({
  open,
  status,
  onStatus,
  onDone,
}: FirstRunSetupProps) {
  const titleId = useId();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onDone();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onDone]);

  if (!open) return null;

  const setupPasteAccess = async () => {
    setBusy(true);
    setMessage(null);
    try {
      let next = await prepareInputHelperForPaste();
      onStatus(next);

      if (next.daemon && next.canInject && next.accessConfigured !== false) {
        setMessage("Auto-paste is ready — enable it anytime in Settings.");
      } else if (next.daemon && next.canInject && next.accessConfigured === false) {
        setMessage(
          next.detail ||
            "Helper can inject temporarily, but permanent access (group/udev) still needs Grant.",
        );
      } else {
        setMessage(next.detail || "Could not finish setup.");
      }
    } catch (error) {
      setMessage(setupError(error));
    } finally {
      setBusy(false);
    }
  };

  const ready = Boolean(
    status?.daemon && status.canInject && status.accessConfigured !== false,
  );
  const isFlatpak = Boolean(status?.flatpak);

  return (
    <div className="macro-dialog-backdrop first-run-backdrop">
      <div
        className="macro-dialog first-run-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <h3 id={titleId}>Welcome to emobie</h3>
        <div className="first-run-body">
          <p className="first-run-copy">
            Optional setup for auto-paste. The helper runs as your user and
            only pastes when you copy an emoji or macro with "Auto-paste on
            copy" enabled in Settings — it does not watch your keyboard.
          </p>

          {isFlatpak ? (
            <p className="first-run-copy">
              Flatpak installs the host input helper automatically when you
              continue — one admin Grant prompt for paste access.
            </p>
          ) : null}

          <button
            type="button"
            className="btn primary first-run-cta"
            disabled={busy || ready}
            onClick={() => void setupPasteAccess()}
          >
            {ready
              ? "Auto-paste ready"
              : busy
                ? "Working…"
                : "Set up auto-paste"}
          </button>

          <p className="first-run-copy">
            {isFlatpak
              ? "Starts the host input helper when available and may ask once for admin approval."
              : "Starts the input helper and may ask once for admin approval. Session ACLs usually mean no logout."}
          </p>

          {message ? (
            <p
              className={
                ready ? "first-run-status" : "first-run-status is-error"
              }
            >
              {message}
            </p>
          ) : null}
        </div>

        <div className="first-run-footer">
          <button type="button" className="btn" disabled={busy} onClick={onDone}>
            Skip for now
          </button>
          <button
            type="button"
            className="btn primary"
            disabled={busy}
            onClick={onDone}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
