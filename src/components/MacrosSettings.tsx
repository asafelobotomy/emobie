import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Macro, MacroTriggerMode } from "../types/preferences";
import type { InputHelperStatus } from "../lib/inputHelper";
import {
  exportMacrosYaml,
  importMacrosYaml,
} from "../lib/macroYaml";

type MacrosSettingsProps = {
  macros: Macro[];
  showShortcodeMacros: boolean;
  expandAsYouType: boolean;
  expandTriggerMode: MacroTriggerMode;
  inputStatus: InputHelperStatus | null;
  onShowShortcodes: (value: boolean) => void;
  onExpandAsYouType: (value: boolean) => void;
  onExpandTriggerMode: (value: MacroTriggerMode) => void;
  onSetMacros: (macros: Macro[]) => void;
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

export function MacrosSettings({
  macros,
  showShortcodeMacros,
  expandAsYouType,
  expandTriggerMode,
  inputStatus,
  onShowShortcodes,
  onExpandAsYouType,
  onExpandTriggerMode,
  onSetMacros,
  onInputStatus,
}: MacrosSettingsProps) {
  const [ioMessage, setIoMessage] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);

  const exportYaml = () => {
    const text = exportMacrosYaml(macros);
    const blob = new Blob([text], { type: "text/yaml" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "emobie-macros.yaml";
    anchor.click();
    URL.revokeObjectURL(url);
    setIoMessage("Exported macros YAML.");
  };

  const importYaml = async (file: File | null) => {
    if (!file) return;
    try {
      const text = await file.text();
      const result = importMacrosYaml(text, macros);
      onSetMacros(result.macros);
      setIoMessage(
        `Imported ${result.imported} match(es)` +
          (result.skipped ? `, skipped ${result.skipped}` : ""),
      );
    } catch (error) {
      setIoMessage(
        error instanceof Error ? error.message : "Import failed.",
      );
    }
  };

  const startHelper = async () => {
    setStarting(true);
    try {
      const status = await invoke<InputHelperStatus>(
        "input_helper_ensure_started",
      );
      onInputStatus(status);
      setIoMessage(status.daemon ? "Input helper started." : status.detail);
    } catch (error) {
      setIoMessage(
        error instanceof Error ? error.message : "Could not start helper.",
      );
    } finally {
      setStarting(false);
    }
  };

  const canListen = Boolean(inputStatus?.canListen);
  const daemonReady = Boolean(inputStatus?.daemon);

  const setMode = (mode: MacroTriggerMode) => {
    onExpandTriggerMode(mode);
  };

  return (
    <div className="macros-settings">
      <h3 className="settings-section-title">Macros</h3>
      <p className="settings-hint settings-hint-block">
        Add and edit macros from the Macros category (+). Import and export
        stay here. Auto-paste lives under Clipboard above.
      </p>

      <p className="settings-hint settings-hint-block">
        {helperStatusLabel(inputStatus)}
      </p>
      {!daemonReady ? (
        <div className="settings-actions macros-io-actions">
          <button
            type="button"
            className="btn primary"
            disabled={starting}
            onClick={() => void startHelper()}
          >
            {starting ? "Starting…" : "Start input helper"}
          </button>
        </div>
      ) : null}
      {daemonReady && !canListen ? (
        <p className="settings-hint settings-hint-block">
          As-you-type needs keyboard access. On the host run{" "}
          <code>pkexec /usr/share/emobie/setup-input-access.sh</code> (or{" "}
          <code>packaging/setup-input-access.sh</code>), then log out/in.
          Group membership is sensitive.
        </p>
      ) : null}

      <div className="settings-row settings-toggle-row">
        <label htmlFor="show-shortcodes">Show emoji shortcodes</label>
        <input
          id="show-shortcodes"
          type="checkbox"
          checked={showShortcodeMacros}
          onChange={(event) => onShowShortcodes(event.target.checked)}
        />
      </div>

      <div className="settings-row settings-toggle-row">
        <label htmlFor="expand-as-you-type">Expand as you type</label>
        <input
          id="expand-as-you-type"
          type="checkbox"
          checked={expandAsYouType}
          disabled={!canListen && !expandAsYouType}
          title={
            !canListen
              ? "Needs keyboard access (emobie-input group). Run setup, then log out/in."
              : undefined
          }
          onChange={(event) => {
            if (event.target.checked && !canListen) return;
            onExpandAsYouType(event.target.checked);
          }}
        />
      </div>
      <p className="settings-hint settings-hint-block">
        Watches keystrokes to expand triggers. Off by default. Requires
        emobie-inputd with keyboard access
        {!canListen
          ? " — toggle stays off until setup-input-access.sh succeeds and you log out/in."
          : "."}
      </p>

      <fieldset
        className="macro-trigger-mode"
        disabled={!expandAsYouType}
      >
        <legend>Expand when</legend>
        <label className="macro-trigger-option">
          <input
            type="radio"
            name="expand-trigger-mode"
            checked={expandTriggerMode === "immediate"}
            onChange={() => setMode("immediate")}
          />
          <span>
            As you type
            <small>Fires as soon as the trigger is complete</small>
          </span>
        </label>
        <label className="macro-trigger-option">
          <input
            type="radio"
            name="expand-trigger-mode"
            checked={expandTriggerMode === "space"}
            onChange={() => setMode("space")}
          />
          <span>
            After Space
            <small>Waits for Space, then replaces trigger + Space</small>
          </span>
        </label>
      </fieldset>

      <div className="settings-actions macros-io-actions">
        <button type="button" className="btn" onClick={exportYaml}>
          Export YAML
        </button>
        <label className="btn file-btn">
          Import YAML
          <input
            type="file"
            accept=".yaml,.yml,text/yaml,text/plain"
            hidden
            onChange={(event) => {
              const file = event.target.files?.[0] ?? null;
              void importYaml(file);
              event.target.value = "";
            }}
          />
        </label>
      </div>
      {ioMessage ? <p className="settings-hint">{ioMessage}</p> : null}
      {macros.length > 0 ? (
        <p className="settings-hint">
          {macros.length} custom macro{macros.length === 1 ? "" : "s"} saved.
        </p>
      ) : null}
    </div>
  );
}
