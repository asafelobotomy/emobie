import { useState } from "react";
import type { Macro } from "../types/preferences";
import {
  exportMacrosYaml,
  importMacrosYaml,
} from "../lib/macroYaml";

type MacrosSettingsProps = {
  macros: Macro[];
  showShortcodeMacros: boolean;
  autoPasteOnCopy: boolean;
  expandAsYouType: boolean;
  inputStatus: {
    daemon: boolean;
    canInject: boolean;
    canListen: boolean;
    detail: string;
  } | null;
  onShowShortcodes: (value: boolean) => void;
  onAutoPaste: (value: boolean) => void;
  onExpandAsYouType: (value: boolean) => void;
  onSetMacros: (macros: Macro[]) => void;
};

export function MacrosSettings({
  macros,
  showShortcodeMacros,
  autoPasteOnCopy,
  expandAsYouType,
  inputStatus,
  onShowShortcodes,
  onAutoPaste,
  onExpandAsYouType,
  onSetMacros,
}: MacrosSettingsProps) {
  const [ioMessage, setIoMessage] = useState<string | null>(null);

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

  const daemonReady = Boolean(inputStatus?.daemon);
  const canInject = Boolean(inputStatus?.canInject);
  const canListen = Boolean(inputStatus?.canListen);

  return (
    <div className="macros-settings">
      <h3 className="settings-section-title">Macros</h3>
      <p className="settings-hint settings-hint-block">
        Add and edit macros from the Macros category (+). Import and export
        stay here.
      </p>

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
        <label htmlFor="auto-paste">Auto-paste on copy</label>
        <input
          id="auto-paste"
          type="checkbox"
          checked={autoPasteOnCopy}
          disabled={!canInject && !autoPasteOnCopy}
          onChange={(event) => onAutoPaste(event.target.checked)}
        />
      </div>
      {!canInject ? (
        <p className="settings-hint settings-hint-block">
          Auto-paste needs the host input helper (
          {inputStatus?.detail ?? "not available"}).
        </p>
      ) : null}

      <div className="settings-row settings-toggle-row">
        <label htmlFor="expand-as-you-type">Expand as you type</label>
        <input
          id="expand-as-you-type"
          type="checkbox"
          checked={expandAsYouType}
          disabled={!canListen && !expandAsYouType}
          onChange={(event) => {
            if (event.target.checked && !daemonReady) return;
            onExpandAsYouType(event.target.checked);
          }}
        />
      </div>
      <p className="settings-hint settings-hint-block">
        Watches keystrokes to expand triggers. Requires emobie-inputd.
        {inputStatus ? ` ${inputStatus.detail}` : ""}
      </p>

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
