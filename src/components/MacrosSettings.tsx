import { useState } from "react";
import type { Macro } from "../types/preferences";
import {
  exportMacrosYaml,
  importMacrosYaml,
} from "../lib/macroYaml";

type MacrosSettingsProps = {
  macros: Macro[];
  favorites: string[];
  favoriteEmojiMacros: boolean;
  onFavoriteEmojiMacros: (value: boolean) => void;
  onSetMacros: (macros: Macro[]) => void;
};

export function MacrosSettings({
  macros,
  favorites,
  favoriteEmojiMacros,
  onFavoriteEmojiMacros,
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

  return (
    <div className="macros-settings">
      <h3 className="settings-section-title">Macros</h3>
      <p className="settings-hint settings-hint-block">
        Add and edit macros from the Macros category (+). Text expansion
        options live in the section above.
      </p>

      <div className="settings-row settings-toggle-row">
        <label htmlFor="favorite-emoji-macros">
          Add favorited emojis as macros
        </label>
        <input
          id="favorite-emoji-macros"
          type="checkbox"
          checked={favoriteEmojiMacros}
          onChange={(event) => onFavoriteEmojiMacros(event.target.checked)}
        />
      </div>
      <p className="settings-hint settings-hint-block">
        When on, shortcodes and emoticons for emojis in your Favorites appear
        under Macros. Choose <code>:)</code> vs <code>:-)</code> style in
        Settings → Emoticon style.
      </p>
      {favoriteEmojiMacros && favorites.length === 0 ? (
        <p className="settings-hint settings-hint-block">
          No favorites yet — star some emojis first.
        </p>
      ) : null}

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
