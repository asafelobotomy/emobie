import { useEffect, useId, useRef, useState } from "react";
import { HotkeyCapture } from "./HotkeyCapture";
import type { Macro } from "../types/preferences";
import {
  findHotkeyConflict,
  findTriggerConflict,
} from "../lib/macroHelpers";
import {
  INLINE_MARKERS,
  toggleInlineMarker,
  toggleQuoteLines,
  type InlineMarker,
  type TextSelection,
} from "../lib/textFormatting";

type MacroEditorDialogProps = {
  macros: Macro[];
  summonHotkey: string;
  initial: Macro | null;
  onSave: (macro: Macro) => void;
  onDelete?: (id: string) => void;
  onClose: () => void;
};

type Draft = {
  id: string | null;
  trigger: string;
  expansion: string;
  hotkey: string | null;
};

function toDraft(macro: Macro | null): Draft {
  if (!macro) {
    return {
      id: null,
      trigger: "",
      expansion: "",
      hotkey: null,
    };
  }
  return {
    id: macro.id,
    trigger: macro.trigger,
    expansion: macro.expansion,
    hotkey: macro.hotkey,
  };
}

export function MacroEditorDialog({
  macros,
  summonHotkey,
  initial,
  onSave,
  onDelete,
  onClose,
}: MacroEditorDialogProps) {
  const titleId = useId();
  const firstFieldRef = useRef<HTMLTextAreaElement>(null);
  const [draft, setDraft] = useState<Draft>(() => toDraft(initial));
  const [formError, setFormError] = useState<string | null>(null);
  const [pendingSelection, setPendingSelection] =
    useState<TextSelection | null>(null);
  const editing = Boolean(initial);

  useEffect(() => {
    if (!pendingSelection) return;
    const field = firstFieldRef.current;
    if (!field) return;
    field.focus();
    field.setSelectionRange(pendingSelection.start, pendingSelection.end);
    setPendingSelection(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft.expansion, pendingSelection]);

  const currentSelection = (): TextSelection => {
    const field = firstFieldRef.current;
    return {
      start: field?.selectionStart ?? draft.expansion.length,
      end: field?.selectionEnd ?? draft.expansion.length,
    };
  };

  const applyInlineMarker = (marker: InlineMarker) => {
    const { text, selection } = toggleInlineMarker(
      draft.expansion,
      currentSelection(),
      INLINE_MARKERS[marker],
    );
    setDraft({ ...draft, expansion: text });
    setPendingSelection(selection);
  };

  const applyQuote = () => {
    const { text, selection } = toggleQuoteLines(
      draft.expansion,
      currentSelection(),
    );
    setDraft({ ...draft, expansion: text });
    setPendingSelection(selection);
  };

  useEffect(() => {
    firstFieldRef.current?.focus();
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const save = () => {
    const triggerError = findTriggerConflict(
      macros,
      draft.trigger,
      draft.id ?? undefined,
    );
    if (triggerError) {
      setFormError(triggerError);
      return;
    }
    if (!draft.expansion) {
      setFormError("Expansion is required.");
      return;
    }
    const hotkeyError = findHotkeyConflict(
      macros,
      summonHotkey,
      draft.hotkey,
      draft.id ?? undefined,
    );
    if (hotkeyError) {
      setFormError(hotkeyError);
      return;
    }
    onSave({
      id: draft.id ?? crypto.randomUUID(),
      trigger: draft.trigger.trim(),
      expansion: draft.expansion,
      hotkey: draft.hotkey,
      // Expansion on/off is global (Settings → Text expansion).
      enabled: initial?.enabled ?? true,
    });
  };

  return (
    <div className="macro-dialog-backdrop" onClick={onClose}>
      <div
        className="macro-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(event) => event.stopPropagation()}
      >
        <h3 id={titleId}>{editing ? "Edit macro" : "Add macro"}</h3>
        <div className="settings-row">
          <label htmlFor="macro-dialog-expansion">Output</label>
          <div className="macro-format-toolbar" role="toolbar" aria-label="Formatting">
            <button type="button" className="macro-format-btn" title="Bold" onClick={() => applyInlineMarker("bold")}>
              <strong>B</strong>
            </button>
            <button type="button" className="macro-format-btn" title="Italic" onClick={() => applyInlineMarker("italic")}>
              <em>I</em>
            </button>
            <button type="button" className="macro-format-btn" title="Strikethrough" onClick={() => applyInlineMarker("strike")}>
              <s>S</s>
            </button>
            <button type="button" className="macro-format-btn" title="Code" onClick={() => applyInlineMarker("code")}>
              <code>{"<>"}</code>
            </button>
            <button type="button" className="macro-format-btn" title="Quote" onClick={applyQuote}>
              &ldquo;&rdquo;
            </button>
          </div>
          <textarea
            ref={firstFieldRef}
            id="macro-dialog-expansion"
            rows={4}
            value={draft.expansion}
            onChange={(event) =>
              setDraft({ ...draft, expansion: event.target.value })
            }
            placeholder="😊 or any text"
          />
          <p className="settings-hint settings-hint-block">
            Markdown-style markup (e.g. <code>**bold**</code>) — how it
            renders depends on where you paste it.
          </p>
        </div>
        <div className="settings-row">
          <label htmlFor="macro-dialog-trigger">Trigger</label>
          <input
            id="macro-dialog-trigger"
            value={draft.trigger}
            onChange={(event) =>
              setDraft({ ...draft, trigger: event.target.value })
            }
            placeholder=".hi or :) or :sig"
          />
        </div>
        <p className="settings-hint settings-hint-block">
          With Settings → Expand as you type on, typing the trigger anywhere
          replaces it with the expansion (e.g. <code>.hi</code> then Space).
        </p>
        <HotkeyCapture
          value={draft.hotkey ?? ""}
          error={null}
          label="Macro hotkey (optional)"
          hint="Optional. Leave empty to copy only from the Macros list."
          allowClear
          onChange={(hotkey) =>
            setDraft({ ...draft, hotkey: hotkey || null })
          }
        />
        {formError ? <p className="settings-error">{formError}</p> : null}
        <div className="settings-actions">
          {editing && onDelete && draft.id ? (
            <button
              type="button"
              className="btn danger"
              onClick={() => onDelete(draft.id!)}
            >
              Delete
            </button>
          ) : null}
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn primary" onClick={save}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
