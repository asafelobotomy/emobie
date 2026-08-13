import { useState } from "react";
import type { MacroEntry } from "../lib/macros";
import type { Macro } from "../types/preferences";
import { MacroEditorDialog } from "./MacroEditorDialog";

type MacroListProps = {
  macros: MacroEntry[];
  customMacros: Macro[];
  summonHotkey: string;
  flashKey: string | null;
  emptyMessage: string;
  onCopy: (expansion: string, flashKey: string) => void;
  onUpsert: (macro: Macro) => void;
  onRemove: (id: string) => void;
};

function previewExpansion(text: string): string {
  const single = text.replace(/\s+/g, " ").trim();
  if (single.length <= 64) return single;
  return `${single.slice(0, 61)}…`;
}

export function MacroList({
  macros,
  customMacros,
  summonHotkey,
  flashKey,
  emptyMessage,
  onCopy,
  onUpsert,
  onRemove,
}: MacroListProps) {
  const [editor, setEditor] = useState<Macro | null | "new">(null);

  return (
    <div className="main-pane macro-list">
      <div className="macro-list-toolbar">
        <span className="macro-list-count">
          {macros.length} macro{macros.length === 1 ? "" : "s"}
        </span>
        <button
          type="button"
          className="btn primary macro-add-btn"
          aria-label="Add macro"
          title="Add macro"
          onClick={() => setEditor("new")}
        >
          +
        </button>
      </div>

      {macros.length === 0 ? (
        <p className="empty-state">{emptyMessage}</p>
      ) : (
        <div className="macro-card-grid" role="list">
          {macros.map((macro) => (
            <button
              key={macro.id}
              type="button"
              role="listitem"
              className={
                flashKey === macro.id ? "macro-card flash" : "macro-card"
              }
              onClick={() => onCopy(macro.expansion, macro.id)}
              onContextMenu={(event) => {
                if (macro.source !== "custom") return;
                event.preventDefault();
                setEditor({
                  id: macro.id,
                  trigger: macro.trigger,
                  expansion: macro.expansion,
                  hotkey: macro.hotkey,
                  enabled: macro.enabled,
                });
              }}
              title={
                macro.source === "custom"
                  ? "Click to copy · Right-click to edit"
                  : macro.expansion
              }
            >
              <span className="macro-card-output">
                {previewExpansion(macro.expansion)}
              </span>
              <span className="macro-card-trigger">{macro.trigger}</span>
              {macro.source === "shortcode" ? (
                <span className="macro-badge">emoji</span>
              ) : macro.hotkey ? (
                <span className="macro-badge">{macro.hotkey}</span>
              ) : null}
            </button>
          ))}
        </div>
      )}

      {editor !== null ? (
        <MacroEditorDialog
          macros={customMacros}
          summonHotkey={summonHotkey}
          initial={editor === "new" ? null : editor}
          onSave={(macro) => {
            onUpsert(macro);
            setEditor(null);
          }}
          onDelete={
            editor === "new"
              ? undefined
              : (id) => {
                  onRemove(id);
                  setEditor(null);
                }
          }
          onClose={() => setEditor(null)}
        />
      ) : null}
    </div>
  );
}
