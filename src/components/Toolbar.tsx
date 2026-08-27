import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  CloseIcon,
  MinimizeIcon,
  PinIcon,
  SearchIcon,
  SettingsIcon,
} from "./Icons";

type ToolbarProps = {
  query: string;
  onQueryChange: (value: string) => void;
  pinned: boolean;
  onTogglePin: () => void;
  onOpenSettings: () => void;
  frameless: boolean;
  trayUnavailable?: boolean;
};

export function Toolbar({
  query,
  onQueryChange,
  pinned,
  onTogglePin,
  onOpenSettings,
  frameless,
  trayUnavailable = false,
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
    if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return;
    event.preventDefault();
    void getCurrentWindow()
      .startDragging()
      .catch((error) => {
        console.error("Failed to start window drag", error);
      });
  };

  const minimizeWindow = () => {
    void getCurrentWindow()
      .minimize()
      .catch((error) => {
        console.error("Failed to minimize window", error);
      });
  };

  const closeWindow = () => {
    void getCurrentWindow()
      .close()
      .catch((error) => {
        console.error("Failed to close window", error);
      });
  };

  const closeLabel = trayUnavailable ? "Close" : "Hide to tray";

  return (
    <header
      className={[
        "toolbar",
        frameless ? "toolbar-draggable" : "",
        showSearchField ? "is-searching" : "",
      ]
        .filter(Boolean)
        .join(" ")}
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

      <div className="toolbar-cluster">
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

        <div className="toolbar-secondary-actions">
          <button
            type="button"
            className="icon-btn pin"
            title={
              pinned
                ? "Unpin"
                : "Pin above windows (Plasma Wayland + X11; limited elsewhere)"
            }
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
      </div>

      <div className="toolbar-window-actions">
        <button
          type="button"
          className="icon-btn window-btn window-btn-minimize"
          title="Minimize"
          aria-label="Minimize"
          data-tauri-drag-region="false"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={minimizeWindow}
        >
          <MinimizeIcon />
        </button>
        <button
          type="button"
          className="icon-btn window-btn window-btn-close"
          title={closeLabel}
          aria-label={closeLabel}
          data-tauri-drag-region="false"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={closeWindow}
        >
          <CloseIcon />
        </button>
      </div>
    </header>
  );
}
