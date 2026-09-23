import { useEffect, useState } from "react";
import type { InputHelperStatus } from "../lib/inputHelper";
import {
  expandReady,
  prepareInputHelperForExpand,
  setInputHelperKeyboardRead,
} from "../lib/inputHelperClient";
import { errorMessage } from "../lib/errorMessage";
import { normalizeExcludedApps } from "../lib/normalizePreferences";
import type { Preferences } from "../types/preferences";

type TextExpansionSettingsProps = {
  prefs: Preferences;
  inputStatus: InputHelperStatus | null;
  updatePrefs: (patch: Partial<Preferences>) => void;
  onInputStatus: (status: InputHelperStatus) => void;
  /** After Grant restarts the helper, re-send macros and options. */
  onHelperReconcile?: () => void;
};

function statusLine(status: InputHelperStatus | null, on: boolean): string | null {
  if (!on) return null;
  if (!status) return "Checking the input helper…";
  if (expandReady(status)) return "Watching for triggers.";
  if (!status.daemon) return status.detail;
  if (!status.keyboardReadConfigured || !status.canListen) {
    return "Keyboard access is missing — turn Expand as you type off and on again to grant it.";
  }
  if (!status.canInject) {
    return "Text injection is unavailable (need writable /dev/uinput). Use Grant under Clipboard.";
  }
  return status.detail;
}

export function TextExpansionSettings({
  prefs,
  inputStatus,
  updatePrefs,
  onInputStatus,
  onHelperReconcile,
}: TextExpansionSettingsProps) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [excludedDraft, setExcludedDraft] = useState(
    prefs.expandExcludedApps.join("\n"),
  );
  const on = prefs.expandAsYouType;

  useEffect(() => {
    setExcludedDraft(prefs.expandExcludedApps.join("\n"));
  }, [prefs.expandExcludedApps]);

  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setMessage(null);
    try {
      await work();
    } catch (error) {
      setMessage(errorMessage(error, "Keyboard access setup failed."));
    } finally {
      setBusy(false);
    }
  };

  const enable = () =>
    run(async () => {
      const status = await prepareInputHelperForExpand();
      onInputStatus(status);
      if (!expandReady(status)) {
        setMessage(status.detail || "Keyboard access was not granted.");
        return;
      }
      updatePrefs({ expandAsYouType: true });
      onHelperReconcile?.();
      setMessage("Text expansion is on.");
    });

  const removeAccess = () =>
    run(async () => {
      const status = await setInputHelperKeyboardRead(false);
      onInputStatus(status);
      onHelperReconcile?.();
      setMessage("Keyboard access removed.");
    });

  const saveExcluded = () => {
    const apps = normalizeExcludedApps(excludedDraft.split("\n"));
    updatePrefs({ expandExcludedApps: apps });
    setExcludedDraft(apps.join("\n"));
  };

  const status = statusLine(inputStatus, on);

  return (
    <div className="text-expansion-settings">
      <h3 className="settings-section-title">Text expansion</h3>

      <div className="settings-row settings-toggle-row">
        <label htmlFor="expand-as-you-type">Expand as you type</label>
        <input
          id="expand-as-you-type"
          type="checkbox"
          checked={on}
          disabled={busy}
          onChange={(event) => {
            if (event.target.checked) void enable();
            else updatePrefs({ expandAsYouType: false });
          }}
        />
      </div>
      <p className="settings-hint settings-hint-block">
        Type a macro's trigger anywhere and it is replaced by the macro. This
        lets emobie's helper read your keyboard — and mouse clicks, only to
        notice the cursor moved — while it is on. Turning it on asks once for
        admin approval. It pauses on the lock screen.
      </p>
      {status ? <p className="settings-hint settings-hint-block">{status}</p> : null}

      {!on && inputStatus?.keyboardReadConfigured ? (
        <div className="settings-actions macros-io-actions">
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => void removeAccess()}
          >
            {busy ? "Working…" : "Remove keyboard access"}
          </button>
        </div>
      ) : null}

      <fieldset className="macro-trigger-mode" disabled={!on}>
        <legend>Expand when</legend>
        <label className="macro-trigger-option">
          <input
            type="radio"
            name="expand-trigger-mode"
            id="expand-trigger-space"
            checked={prefs.expandTriggerMode === "space"}
            onChange={() => updatePrefs({ expandTriggerMode: "space" })}
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
            id="expand-trigger-immediate"
            checked={prefs.expandTriggerMode === "immediate"}
            onChange={() => updatePrefs({ expandTriggerMode: "immediate" })}
          />
          <span>
            As soon as complete
            <small>Fires the moment the trigger finishes</small>
          </span>
        </label>
      </fieldset>

      {on && prefs.expandTriggerMode === "space" ? (
        <div className="settings-row settings-toggle-row">
          <label htmlFor="expand-keep-space">Keep Space after expansion</label>
          <input
            id="expand-keep-space"
            type="checkbox"
            checked={prefs.expandKeepTriggerSpace}
            onChange={(event) =>
              updatePrefs({ expandKeepTriggerSpace: event.target.checked })
            }
          />
        </div>
      ) : null}

      {on ? (
        <>
          <label className="settings-label" htmlFor="expand-excluded-apps">
            Never expand in these apps
          </label>
          <textarea
            id="expand-excluded-apps"
            rows={4}
            value={excludedDraft}
            onChange={(event) => setExcludedDraft(event.target.value)}
            onBlur={saveExcluded}
          />
          <p className="settings-hint settings-hint-block">
            One app per line; part of the app id is enough (<code>keepassxc</code>).
            Works for X11 apps everywhere, and for all apps on GNOME with the
            Focused Window D-Bus extension. Plasma Wayland apps can't be told
            apart yet.
          </p>
        </>
      ) : null}

      {message ? <p className="settings-hint">{message}</p> : null}
    </div>
  );
}
