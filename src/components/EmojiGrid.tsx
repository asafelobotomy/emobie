import { EmojiButton } from "./EmojiButton";
import type { EmobieEmoji } from "../data/loadEmojis";

type EmojiGridProps = {
  emojis: EmobieEmoji[];
  flashKey: string | null;
  favorites: string[];
  emptyMessage?: string;
  onCopy: (emoji: string) => void;
  onToggleFavorite: (hexcode: string) => void;
};

export function EmojiGrid({
  emojis,
  flashKey,
  favorites,
  emptyMessage = "No emojis match your search.",
  onCopy,
  onToggleFavorite,
}: EmojiGridProps) {
  if (emojis.length === 0) {
    return <p className="recent-empty">{emptyMessage}</p>;
  }

  const favoriteSet = new Set(favorites);

  return (
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
  );
}
