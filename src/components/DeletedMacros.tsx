import { useState } from "react";
import type { Macro, DeletedMacro } from "../types/preferences";
import { purgeDeleted, restoreDeletedMacro } from "../lib/deletedMacros";

type DeletedMacrosProps = {
  macros: Macro[];
  deleted: DeletedMacro[];
  summonHotkey: string;
  onChange: (patch: { macros?: Macro[]; deletedMacros?: DeletedMacro[] }) => void;
};

function when(ms: number): string {
  return ms > 0 ? new Date(ms).toLocaleDateString() : "";
}

/** Macros deleted from the Macros list, kept so they can be brought back. */
export function DeletedMacros({
  macros,
  deleted,
  summonHotkey,
  onChange,
}: DeletedMacrosProps) {
  const [message, setMessage] = useState<string | null>(null);

  if (deleted.length === 0) return null;

  const restore = (id: string) => {
    const result = restoreDeletedMacro(macros, deleted, id, summonHotkey);
    if ("error" in result) {
      setMessage(result.error);
      return;
    }
    setMessage(null);
    onChange(result);
  };
  const purge = (id: string | null) => {
    setMessage(null);
    onChange({ deletedMacros: purgeDeleted(deleted, id) });
  };

  return (
    <div className="deleted-macros">
      <h4 className="settings-section-title">Recently deleted</h4>
      <p className="settings-hint settings-hint-block">
        Deleted macros stay here until you remove them for good (the newest
        50 are kept).
      </p>
      <ul className="deleted-macro-list">
        {deleted.map((macro) => (
          <li key={macro.id} className="deleted-macro">
            <span className="deleted-macro-text">
              <code>{macro.trigger}</code>
              <span className="settings-hint">
                {macro.expansion.length > 60
                  ? `${macro.expansion.slice(0, 60)}…`
                  : macro.expansion}
                {macro.deletedAt > 0 ? ` · deleted ${when(macro.deletedAt)}` : ""}
              </span>
            </span>
            <button type="button" className="btn" onClick={() => restore(macro.id)}>
              Restore
            </button>
            <button
              type="button"
              className="btn danger"
              onClick={() => purge(macro.id)}
            >
              Delete for good
            </button>
          </li>
        ))}
      </ul>
      {message ? <p className="settings-hint" role="alert">{message}</p> : null}
      <div className="settings-actions">
        <button type="button" className="btn danger" onClick={() => purge(null)}>
          Delete all for good
        </button>
      </div>
    </div>
  );
}
