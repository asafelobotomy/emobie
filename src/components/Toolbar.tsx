import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
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

  const beginWindowDrag = (event: React.MouseEvent) => {
    if (!frameless || event.button !== 0) return;
    // Ignore modifier-modified clicks so text selection / OS gestures stay free.
    if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return;
    event.preventDefault();
    void getCurrentWindow()
      .startDragging()
      .catch((error) => {
        console.error("Failed to start window drag", error);
      });
  };

  return (
    <header
      className={`toolbar${frameless ? " toolbar-draggable" : ""}`}
      {...(frameless ? { "data-tauri-drag-region": "deep" } : {})}
      onMouseDown={frameless ? beginWindowDrag : undefined}
    >
      <div
        className="brand"
        title={frameless ? "Drag to move emobie" : "emobie"}
        aria-label="emobie"
      >
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
            data-tauri-drag-region="false"
            onMouseDown={(event) => event.stopPropagation()}
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
            data-tauri-drag-region="false"
            onMouseDown={(event) => event.stopPropagation()}
            onClick={() => setSearchOpen(true)}
          >
            <SearchIcon />
          </button>
        )}
      </div>

      <div className="toolbar-actions">
        <button
          type="button"
          className="icon-btn pin"
          title={pinned ? "Unpin" : "Pin above windows"}
          aria-label={pinned ? "Unpin" : "Pin above windows"}
          aria-pressed={pinned}
          data-tauri-drag-region="false"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={onTogglePin}
        >
          <PinIcon />
        </button>
        <button
          type="button"
          className="icon-btn"
          title="Settings"
          aria-label="Settings"
          data-tauri-drag-region="false"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={onOpenSettings}
        >
          <SettingsIcon />
        </button>
      </div>
    </header>
  );
}
