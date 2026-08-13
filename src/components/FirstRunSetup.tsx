import { useEffect, useId, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { InputHelperStatus } from "../lib/inputHelper";

type FirstRunSetupProps = {
  open: boolean;
  status: InputHelperStatus | null;
  onStatus: (status: InputHelperStatus) => void;
  onDone: () => void;
};

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

  const startHelper = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const next = await invoke<InputHelperStatus>(
        "input_helper_ensure_started",
      );
      onStatus(next);
      setMessage(
        next.daemon
          ? "Input helper is running."
          : next.detail || "Could not start the helper.",
      );
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "Could not start the helper.",
      );
    } finally {
      setBusy(false);
    }
  };

  const runAccessSetup = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const detail = await invoke<string>("input_helper_run_access_setup");
      setMessage(detail);
      const next = await invoke<InputHelperStatus>("input_helper_status");
      onStatus(next);
    } catch (error) {
      setMessage(
        error instanceof Error
          ? error.message
          : "Keyboard access setup failed or was cancelled.",
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="macro-dialog-backdrop first-run-backdrop">
      <div
        className="macro-dialog first-run-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <h3 id={titleId}>Welcome to emobie</h3>
        <p className="settings-hint settings-hint-block">
          Optional setup for paste and text expansion. The helper runs as your
          user and only watches keys after you enable Expand as you type.
        </p>

        <ol className="first-run-steps">
          <li>
            <strong>Start input helper</strong>
            <p className="settings-hint">
              Private socket for auto-paste and expansion.
            </p>
            <button
              type="button"
              className="btn primary"
              disabled={busy || Boolean(status?.daemon)}
              onClick={() => void startHelper()}
            >
              {status?.daemon ? "Helper running" : "Start helper"}
            </button>
          </li>
          <li>
            <strong>Keyboard access</strong> (expand as you type)
            <p className="settings-hint">
              Admin prompt to join emobie-input, then log out/in. Skip if you
              only need copy/paste.
            </p>
            <button
              type="button"
              className="btn"
              disabled={busy}
              onClick={() => void runAccessSetup()}
            >
              Grant keyboard access
            </button>
            {status?.daemon && !status.canListen ? (
              <p className="settings-hint">
                Helper is up; finish keyboard setup and re-login if needed.
              </p>
            ) : null}
          </li>
        </ol>

        {message ? <p className="settings-hint">{message}</p> : null}

        <div className="settings-actions">
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
