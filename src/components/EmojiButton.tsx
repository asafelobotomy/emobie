type EmojiButtonProps = {
  emoji: string;
  label: string;
  flashing?: boolean;
  favorited?: boolean;
  onCopy: (emoji: string) => void;
  onToggleFavorite?: () => void;
};

export function EmojiButton({
  emoji,
  label,
  flashing,
  favorited,
  onCopy,
  onToggleFavorite,
}: EmojiButtonProps) {
  const title = onToggleFavorite
    ? `${label} · ${favorited ? "Right-click to unfavorite" : "Right-click to favorite"}`
    : label;

  return (
    <button
      type="button"
      role="listitem"
      className={`emoji-btn${flashing ? " flash" : ""}${favorited ? " favorited" : ""}`}
      title={title}
      aria-label={title}
      onClick={() => onCopy(emoji)}
      onContextMenu={(event) => {
        if (!onToggleFavorite) return;
        event.preventDefault();
        onToggleFavorite();
      }}
      onKeyDown={(event) => {
        if (!onToggleFavorite) return;
        if (
          event.key === "ContextMenu" ||
          (event.key === "F10" && event.shiftKey)
        ) {
          event.preventDefault();
          onToggleFavorite();
        }
      }}
    >
      {emoji}
    </button>
  );
}
