import { useEffect, useRef, useState } from "react";
import { PinIcon, SearchIcon, SettingsIcon } from "./Icons";

type ToolbarProps = {
  query: string;
  onQueryChange: (value: string) => void;
  pinned: boolean;
  onTogglePin: () => void;
  onOpenSettings: () => void;
  frameless: boolean;
};

export function Toolbar({
  query,
  onQueryChange,
  pinned,
  onTogglePin,
  onOpenSettings,
  frameless,
}: ToolbarProps) {
  const [searchOpen, setSearchOpen] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);
  const showSearchField = searchOpen || query.length > 0;
  const showActions = !showSearchField;

  useEffect(() => {
    if (showSearchField) {
      searchRef.current?.focus();
    }
  }, [showSearchField]);

  const closeSearchIfEmpty = () => {
    if (query.length === 0) {
      setSearchOpen(false);
    }
  };

  return (
    <header
      className="toolbar"
      {...(frameless ? { "data-tauri-drag-region": "" } : {})}
    >
      <div className="brand" title="emobie" aria-label="emobie">
        <img
          className="brand-mark"
          src="/emobie-icon.png"
          alt=""
          width={22}
          height={22}
          draggable={false}
        />
        <span className="brand-name">emobie</span>
      </div>

      <div className="toolbar-search">
        {showSearchField ? (
          <input
            ref={searchRef}
            className="search"
            type="search"
            placeholder="Search…"
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            onBlur={closeSearchIfEmpty}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                if (query.length > 0) {
                  onQueryChange("");
                }
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
      </div>

      {showActions ? (
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
