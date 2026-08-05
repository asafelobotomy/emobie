import { EmojiButton } from "./EmojiButton";

type RecentStripProps = {
  recents: string[];
  flashKey: string | null;
  status: string | null;
  statusError?: boolean;
  onCopy: (emoji: string) => void;
};

export function RecentStrip({
  recents,
  flashKey,
  status,
  statusError,
  onCopy,
}: RecentStripProps) {
  return (
    <div className="recent-strip">
      <span className="recent-label" title="Recent">
        Rec
      </span>
      <div className="recent-list">
        {recents.length === 0 ? (
          <span className="recent-empty">None yet</span>
        ) : (
          recents.map((emoji) => (
            <EmojiButton
              key={emoji}
              emoji={emoji}
              label={`Recent ${emoji}`}
              flashing={flashKey === emoji}
              onCopy={onCopy}
            />
          ))
        )}
      </div>
      {status ? (
        <span className={`status${statusError ? " error" : ""}`}>{status}</span>
      ) : null}
    </div>
  );
}
