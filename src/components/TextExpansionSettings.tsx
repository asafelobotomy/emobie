import { invoke } from "@tauri-apps/api/core";
import type { MacroTriggerMode } from "../types/preferences";
import type { InputHelperStatus } from "../lib/inputHelper";
import { useState } from "react";

type TextExpansionSettingsProps = {
  expandAsYouType: boolean;
  expandTriggerMode: MacroTriggerMode;
  expandKeepTriggerSpace: boolean;
  inputStatus: InputHelperStatus | null;
  onExpandAsYouType: (value: boolean) => void;
  onExpandTriggerMode: (value: MacroTriggerMode) => void;
  onExpandKeepTriggerSpace: (value: boolean) => void;
  onInputStatus: (status: InputHelperStatus) => void;
};

function helperStatusLabel(status: InputHelperStatus | null): string {
  if (!status) return "Checking input helper…";
  if (status.daemon && status.canListen) {
    return `Helper running (listen + paste). ${status.detail}`;
  }
  if (status.daemon && !status.canListen) {
    return `Helper running, but keyboard access is missing. ${status.detail}`;
  }
  return status.detail;
}

export function TextExpansionSettings({
  expandAsYouType,
  expandTriggerMode,
  expandKeepTriggerSpace,
  inputStatus,
  onExpandAsYouType,
  onExpandTriggerMode,
  onExpandKeepTriggerSpace,
  onInputStatus,
}: TextExpansionSettingsProps) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const canListen = Boolean(inputStatus?.canListen);
  const daemonReady = Boolean(inputStatus?.daemon);

  const grantAccess = async (): Promise<InputHelperStatus> => {
    return invoke<InputHelperStatus>("input_helper_run_access_setup");
  };

  /** Start helper, grant access if needed (one Polkit prompt), then enable. */
  const setExpandEnabled = async (enabled: boolean) => {
    if (!enabled) {
      onExpandAsYouType(false);
      setMessage(null);
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      let status = await invoke<InputHelperStatus>("input_helper_ensure_started");
      onInputStatus(status);

      if (!status.canListen) {
        setMessage("Authorizing keyboard access…");
        status = await grantAccess();
        onInputStatus(status);
      }

      if (!status.canListen) {
        setMessage(status.detail);
        return;
      }

      onExpandAsYouType(true);
      setMessage("Text expansion enabled.");
    } catch (error) {
      setMessage(
        typeof error === "string" && error.trim()
          ? error
          : error instanceof Error
            ? error.message
            : "Could not enable expansion.",
      );
    } finally {
      setBusy(false);
    }
  };

  const retryGrant = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const status = await grantAccess();
      onInputStatus(status);
      setMessage(status.detail);
      if (status.canListen && !expandAsYouType) {
        onExpandAsYouType(true);
      }
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "Keyboard access setup failed.",
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="text-expansion-settings">
      <h3 className="settings-section-title">Text expansion</h3>
      <p className="settings-hint settings-hint-block">
        {helperStatusLabel(inputStatus)}
      </p>

      <div className="settings-row settings-toggle-row">
        <label htmlFor="expand-as-you-type">Expand as you type</label>
        <input
          id="expand-as-you-type"
          type="checkbox"
          checked={expandAsYouType}
          disabled={busy}
          title="Starts the helper and may ask once for admin approval to grant keyboard access."
          onChange={(event) => {
            void setExpandEnabled(event.target.checked);
          }}
        />
      </div>
      <p className="settings-hint settings-hint-block">
        Starts emobie-inputd and turns on listening. If keyboard access is
        missing, you get one admin prompt — no logout when session ACLs apply.
      </p>

      {daemonReady && !canListen ? (
        <div className="settings-actions macros-io-actions">
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => void retryGrant()}
          >
            {busy ? "Working…" : "Grant keyboard access"}
          </button>
        </div>
      ) : null}

      <fieldset className="macro-trigger-mode" disabled={!expandAsYouType}>
        <legend>Expand when</legend>
        <label className="macro-trigger-option">
          <input
            type="radio"
            name="expand-trigger-mode"
            checked={expandTriggerMode === "space"}
            onChange={() => onExpandTriggerMode("space")}
          />
          <span>
            After Space
            <small>
              Type a trigger then Space — e.g. <code>.hi</code> then Space
            </small>
          </span>
        </label>
        <label className="macro-trigger-option">
          <input
            type="radio"
            name="expand-trigger-mode"
            checked={expandTriggerMode === "immediate"}
            onChange={() => onExpandTriggerMode("immediate")}
          />
          <span>
            As soon as complete
            <small>Fires the moment the trigger finishes</small>
          </span>
        </label>
      </fieldset>

      {expandAsYouType && expandTriggerMode === "space" ? (
        <div className="settings-row settings-toggle-row">
          <label htmlFor="expand-keep-space">
            Keep Space after expansion
          </label>
          <input
            id="expand-keep-space"
            type="checkbox"
            checked={expandKeepTriggerSpace}
            onChange={(event) =>
              onExpandKeepTriggerSpace(event.target.checked)
            }
          />
        </div>
      ) : null}
      {expandAsYouType && expandTriggerMode === "space" ? (
        <p className="settings-hint settings-hint-block">
          Off: <code>.hi</code> + Space → <code>hiya</code>. On: expands to{" "}
          <code>hiya </code> (Space stays after the text).
        </p>
      ) : null}

      {message ? <p className="settings-hint">{message}</p> : null}
    </div>
  );
}
