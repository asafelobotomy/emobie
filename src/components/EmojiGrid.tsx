import { useEffect, useRef } from "react";
import { EmojiButton } from "./EmojiButton";
import type { emobieEmoji } from "../data/loadEmojis";
import type { ScrollAxis } from "../hooks/useLayoutMode";

type EmojiGridProps = {
  emojis: emobieEmoji[];
  flashKey: string | null;
  favorites: string[];
  scrollAxis: ScrollAxis;
  emptyMessage?: string;
  onCopy: (emoji: string) => void;
  onToggleFavorite: (hexcode: string) => void;
};

export function EmojiGrid({
  emojis,
  flashKey,
  favorites,
  scrollAxis,
  emptyMessage = "No emojis match your search.",
  onCopy,
  onToggleFavorite,
}: EmojiGridProps) {
  const paneRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const pane = paneRef.current;
    if (!pane || scrollAxis !== "x") return;

    const onWheel = (event: WheelEvent) => {
      if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
      event.preventDefault();
      pane.scrollLeft += event.deltaY;
    };

    pane.addEventListener("wheel", onWheel, { passive: false });
    return () => pane.removeEventListener("wheel", onWheel);
  }, [scrollAxis]);

  if (emojis.length === 0) {
    return (
      <div className="main-pane" ref={paneRef}>
        <p className="empty-state">{emptyMessage}</p>
      </div>
    );
  }

  const favoriteSet = new Set(favorites);

  return (
    <div className="main-pane" ref={paneRef}>
      <div className="emoji-grid" role="list">
        {emojis.map((emoji) => (
          <EmojiButton
            key={emoji.hexcode}
            emoji={emoji.emoji}
            label={emoji.label}
            flashing={flashKey === emoji.emoji}
            favorited={favoriteSet.has(emoji.hexcode)}
            onCopy={onCopy}
            onToggleFavorite={() => onToggleFavorite(emoji.hexcode)}
          />
        ))}
      </div>
    </div>
  );
}
