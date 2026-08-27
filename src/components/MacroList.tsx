import { useEffect, useMemo, useRef, useState } from "react";
import { CATEGORIES, type Category } from "../data/loadEmojis";
import type { MacroEntry } from "../lib/macros";
import type { Macro } from "../types/preferences";
import { MacroEditorDialog } from "./MacroEditorDialog";

type MacroListProps = {
  macros: MacroEntry[];
  customMacros: Macro[];
  summonHotkey: string;
  flashKey: string | null;
  emptyMessage: string;
  searchActive?: boolean;
  onCopy: (expansion: string, flashKey: string) => void;
  onUpsert: (macro: Macro) => void;
  onRemove: (id: string) => void;
};

type CategoryGroup = {
  category: Category;
  macros: MacroEntry[];
};

function previewExpansion(text: string): string {
  const single = text.replace(/\s+/g, " ").trim();
  if (single.length <= 48) return single;
  return `${single.slice(0, 45)}…`;
}

function groupShortcodes(macros: MacroEntry[]): CategoryGroup[] {
  const byGroup = new Map<number, MacroEntry[]>();
  for (const macro of macros) {
    const group = macro.group ?? -1;
    const list = byGroup.get(group);
    if (list) list.push(macro);
    else byGroup.set(group, [macro]);
  }

  const grouped: CategoryGroup[] = [];
  for (const category of CATEGORIES) {
    const items = byGroup.get(category.id);
    if (!items?.length) continue;
    grouped.push({ category, macros: items });
    byGroup.delete(category.id);
  }

  if (byGroup.size > 0) {
    const leftovers: MacroEntry[] = [];
    for (const items of byGroup.values()) leftovers.push(...items);
    grouped.push({
      category: {
        id: -999,
        key: "other",
        label: "Other",
        icon: "✨",
      },
      macros: leftovers,
    });
  }

  return grouped;
}

function MacroCard({
  macro,
  flashKey,
  onCopy,
  onEdit,
}: {
  macro: MacroEntry;
  flashKey: string | null;
  onCopy: (expansion: string, flashKey: string) => void;
  onEdit?: (macro: MacroEntry) => void;
}) {
  const custom = macro.source === "custom";
  return (
    <button
      type="button"
      role="listitem"
      className={flashKey === macro.id ? "macro-card flash" : "macro-card"}
      onClick={() => onCopy(macro.expansion, macro.id)}
      onContextMenu={(event) => {
        if (!custom || !onEdit) return;
        event.preventDefault();
        onEdit(macro);
      }}
      title={
        custom ? "Click to copy · Right-click to edit" : macro.expansion
      }
    >
      <span className="macro-card-output">
        {previewExpansion(macro.expansion)}
      </span>
      <span className="macro-card-trigger">{macro.trigger}</span>
      {custom && macro.hotkey ? (
        <span className="macro-badge">{macro.hotkey}</span>
      ) : null}
    </button>
  );
}

function DisclosureButton({
  id,
  open,
  label,
  count,
  icon,
  onToggle,
}: {
  id: string;
  open: boolean;
  label: string;
  count: number;
  icon?: string;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      id={id}
      className="macro-disclosure"
      aria-expanded={open}
      onClick={onToggle}
    >
      <span className="macro-disclosure-chevron" aria-hidden="true">
        {open ? "▾" : "▸"}
      </span>
      {icon ? (
        <span className="macro-disclosure-icon" aria-hidden="true">
          {icon}
        </span>
      ) : null}
      <span className="macro-disclosure-label">{label}</span>
      <span className="macro-list-count">{count}</span>
    </button>
  );
}

export function MacroList({
  macros,
  customMacros,
  summonHotkey,
  flashKey,
  emptyMessage,
  searchActive = false,
  onCopy,
  onUpsert,
  onRemove,
}: MacroListProps) {
  const [editor, setEditor] = useState<Macro | null | "new">(null);
  const [emojiOpen, setEmojiOpen] = useState(false);
  const [openGroups, setOpenGroups] = useState<Set<number>>(() => new Set());
  const wasSearching = useRef(false);

  const { custom, shortcodes } = useMemo(() => {
    const nextCustom: MacroEntry[] = [];
    const nextShortcodes: MacroEntry[] = [];
    for (const macro of macros) {
      if (macro.source === "custom") nextCustom.push(macro);
      else nextShortcodes.push(macro);
    }
    return { custom: nextCustom, shortcodes: nextShortcodes };
  }, [macros]);

  const shortcodeGroups = useMemo(
    () => groupShortcodes(shortcodes),
    [shortcodes],
  );

  useEffect(() => {
    if (searchActive && shortcodes.length > 0) {
      wasSearching.current = true;
      setEmojiOpen(true);
      setOpenGroups(
        new Set(shortcodeGroups.map((group) => group.category.id)),
      );
      return;
    }
    if (wasSearching.current && !searchActive) {
      wasSearching.current = false;
      setEmojiOpen(false);
      setOpenGroups(new Set());
    }
  }, [searchActive, shortcodes.length, shortcodeGroups]);

  const nothingVisible = custom.length === 0 && shortcodes.length === 0;

  const toggleEmojiSection = () => {
    setEmojiOpen((open) => {
      if (open) setOpenGroups(new Set());
      return !open;
    });
  };

  const toggleGroup = (id: number) => {
    setOpenGroups((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="main-pane macro-list">
      <section className="macro-section" aria-labelledby="macro-section-yours">
        <div className="macro-list-toolbar">
          <div className="macro-section-heading">
            <h3 id="macro-section-yours">Your macros</h3>
            <span className="macro-list-count">{custom.length}</span>
          </div>
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
        {custom.length === 0 ? (
          <p
            className={
              nothingVisible ? "empty-state" : "macro-section-empty"
            }
          >
            {nothingVisible
              ? emptyMessage
              : "No custom macros in this view."}
          </p>
        ) : (
          <div className="macro-card-grid" role="list">
            {custom.map((macro) => (
              <MacroCard
                key={macro.id}
                macro={macro}
                flashKey={flashKey}
                onCopy={onCopy}
                onEdit={(entry) =>
                  setEditor({
                    id: entry.id,
                    trigger: entry.trigger,
                    expansion: entry.expansion,
                    hotkey: entry.hotkey,
                    enabled: entry.enabled,
                  })
                }
              />
            ))}
          </div>
        )}
      </section>

      {shortcodes.length > 0 ? (
        <section
          className="macro-section macro-section-emoji"
          aria-labelledby="macro-section-emoji"
        >
          <div className="macro-list-toolbar">
            <DisclosureButton
              id="macro-section-emoji"
              open={emojiOpen}
              label="Emoji macros"
              count={shortcodes.length}
              onToggle={toggleEmojiSection}
            />
          </div>
          {emojiOpen ? (
            <div className="macro-category-list" hidden={!emojiOpen}>
              {shortcodeGroups.map(({ category, macros: items }) => {
                const open = openGroups.has(category.id);
                const panelId = `macro-cat-${category.key}`;
                return (
                  <div key={category.key} className="macro-category">
                    <DisclosureButton
                      id={`${panelId}-btn`}
                      open={open}
                      label={category.label}
                      count={items.length}
                      icon={category.icon}
                      onToggle={() => toggleGroup(category.id)}
                    />
                    {open ? (
                      <div
                        id={panelId}
                        className="macro-card-grid"
                        role="list"
                        aria-labelledby={`${panelId}-btn`}
                        hidden={!open}
                      >
                        {items.map((macro) => (
                          <MacroCard
                            key={macro.id}
                            macro={macro}
                            flashKey={flashKey}
                            onCopy={onCopy}
                          />
                        ))}
                      </div>
                    ) : null}
                  </div>
                );
              })}
            </div>
          ) : null}
        </section>
      ) : null}

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
