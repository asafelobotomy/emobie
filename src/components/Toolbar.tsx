import { useEffect, useRef, useState } from "react";
import { PinIcon, SearchIcon, SettingsIcon } from "./Icons";

type ToolbarProps = {
  query: string;
  onQueryChange: (value: string) => void;
  pinned: boolean;
  onTogglePin: () => void;
  onOpenSettings: () => void;
  compact: boolean;
};

export function Toolbar({
  query,
  onQueryChange,
  pinned,
  onTogglePin,
  onOpenSettings,
  compact,
}: ToolbarProps) {
  const [searchOpen, setSearchOpen] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);
  const showSearchField = !compact || searchOpen || query.length > 0;

  useEffect(() => {
    if (compact && showSearchField) {
      searchRef.current?.focus();
    }
  }, [compact, showSearchField]);

  useEffect(() => {
    if (!compact) {
      setSearchOpen(false);
    }
  }, [compact]);

  return (
    <header className="toolbar">
      <button
        type="button"
        className="brand"
        title={compact ? "Open settings" : "Emobie"}
        aria-label={compact ? "Open settings" : "Emobie"}
        onClick={() => {
          if (compact) onOpenSettings();
        }}
      >
        Emobie
      </button>

      {showSearchField ? (
        <input
          ref={searchRef}
          className="search"
          type="search"
          placeholder="Search emojis…"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onBlur={() => {
            if (compact && query.length === 0) {
              setSearchOpen(false);
            }
          }}
          aria-label="Search emojis"
        />
      ) : (
        <button
          type="button"
          className="icon-btn search-toggle"
          title="Search emojis"
          aria-label="Search emojis"
          onClick={() => setSearchOpen(true)}
        >
          <SearchIcon />
        </button>
      )}

      {!compact ? (
        <div className="toolbar-actions">
          <button
            type="button"
            className="icon-btn pin"
            title={pinned ? "Unpin" : "Pin above windows"}
            aria-label={pinned ? "Unpin" : "Pin above windows"}
            aria-pressed={pinned}
            onClick={onTogglePin}
          >
            <PinIcon />
          </button>
          <button
            type="button"
            className="icon-btn"
            title="Settings"
            aria-label="Settings"
            onClick={onOpenSettings}
          >
            <SettingsIcon />
          </button>
        </div>
      ) : null}
    </header>
  );
}
