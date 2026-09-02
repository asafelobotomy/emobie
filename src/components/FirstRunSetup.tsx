import { useEffect, useId, useState } from "react";
import type { InputHelperStatus } from "../lib/inputHelper";
import { prepareInputHelperForExpand } from "../lib/inputHelperClient";

type FirstRunSetupProps = {
  open: boolean;
  status: InputHelperStatus | null;
  onStatus: (status: InputHelperStatus) => void;
  onDone: () => void;
};

function errorMessage(error: unknown): string {
  if (typeof error === "string" && error.trim()) return error;
  if (error instanceof Error && error.message) return error.message;
  return "Setup failed or was cancelled.";
}

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

  const setupTextExpansion = async () => {
    setBusy(true);
    setMessage(null);
    try {
      let next = await prepareInputHelperForExpand();
      onStatus(next);

      if (next.daemon && next.canListen && next.canInject && next.accessConfigured !== false) {
        setMessage("Text expansion is ready — enable it anytime in Settings.");
      } else if (next.daemon && next.canListen && next.accessConfigured === false) {
        setMessage(
          next.detail ||
            "Helper can listen temporarily, but permanent keyboard access (group/udev) still needs Grant.",
        );
      } else if (next.daemon && next.canListen && !next.canInject) {
        setMessage(
          "Keyboard access OK, but text injection needs a desktop session. Restart emobie-inputd or log out/in, then enable Expand in Settings.",
        );
      } else {
        setMessage(next.detail || "Could not finish setup.");
      }
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  const ready = Boolean(
    status?.daemon &&
      status.canListen &&
      status.canInject &&
      status.accessConfigured !== false,
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
            Optional setup for paste and text expansion. The helper runs as your
            user and only watches keys after you enable Expand as you type.
          </p>

          {isFlatpak ? (
            <p className="first-run-copy">
              Flatpak installs the host input helper automatically when you
              continue — one admin Grant prompt for keyboard access.
            </p>
          ) : null}

          <button
            type="button"
            className="btn primary first-run-cta"
            disabled={busy || ready}
            onClick={() => void setupTextExpansion()}
          >
            {ready
              ? "Text expansion ready"
              : busy
                ? "Working…"
                : "Set up text expansion"}
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
