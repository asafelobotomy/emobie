import { useEffect, useId, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { InputHelperStatus } from "../lib/inputHelper";

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
      let next = await invoke<InputHelperStatus>("input_helper_ensure_started");
      onStatus(next);

      if (!next.canListen) {
        setMessage("Admin prompt next — grant keyboard access to continue.");
        next = await invoke<InputHelperStatus>("input_helper_run_access_setup");
        onStatus(next);
      }

      if (next.daemon && next.canListen) {
        setMessage("Text expansion is ready — enable it anytime in Settings.");
      } else {
        setMessage(next.detail || "Could not finish setup.");
      }
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  const ready = Boolean(status?.daemon && status.canListen);
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
              Flatpak build: install the host helper first (
              <code>bash packaging/install-inputd-user.sh</code>
              ), then continue — Grant uses a host admin prompt.
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
