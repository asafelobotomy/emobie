/**
 * Inline text formatting for macro expansions.
 *
 * The editor toolbar writes a small, canonical Markdown-like syntax directly
 * into the expansion text (**bold**, *italic*, ~~strike~~, `code`, "> quote"
 * line prefixes) — how it renders depends on where it's pasted.
 */

export type InlineMarker = "bold" | "italic" | "strike" | "code";

/** Canonical wrap marker each toolbar button writes into the draft text. */
export const INLINE_MARKERS: Record<InlineMarker, string> = {
  bold: "**",
  italic: "*",
  strike: "~~",
  code: "`",
};

export type TextSelection = { start: number; end: number };

/**
 * Toggle a symmetric inline marker (bold/italic/strike/code) around the
 * current selection — unwraps if the selection is already wrapped, else
 * wraps it. With no selection, inserts an empty pair and places the cursor
 * between them.
 */
export function toggleInlineMarker(
  text: string,
  selection: TextSelection,
  marker: string,
): { text: string; selection: TextSelection } {
  const { start, end } = selection;
  const before = text.slice(Math.max(0, start - marker.length), start);
  const after = text.slice(end, end + marker.length);
  if (marker.length > 0 && before === marker && after === marker) {
    const next =
      text.slice(0, start - marker.length) +
      text.slice(start, end) +
      text.slice(end + marker.length);
    return {
      text: next,
      selection: { start: start - marker.length, end: end - marker.length },
    };
  }
  const selected = text.slice(start, end);
  const next = text.slice(0, start) + marker + selected + marker + text.slice(end);
  return {
    text: next,
    selection: { start: start + marker.length, end: end + selected.length + marker.length },
  };
}

const QUOTE_PREFIX = "> ";

/** Toggle a "> " quote prefix on every line the selection spans. */
export function toggleQuoteLines(
  text: string,
  selection: TextSelection,
): { text: string; selection: TextSelection } {
  const { start, end } = selection;
  const blockStart = text.lastIndexOf("\n", start - 1) + 1;
  const nextBreak = text.indexOf("\n", Math.max(end - 1, blockStart));
  const blockEnd = nextBreak === -1 ? text.length : nextBreak;

  const block = text.slice(blockStart, blockEnd);
  const lines = block.split("\n");
  const allQuoted = lines.every((line) => line.startsWith(QUOTE_PREFIX) || line === "");
  const nextLines = lines.map((line) => {
    if (allQuoted) return line.startsWith(QUOTE_PREFIX) ? line.slice(QUOTE_PREFIX.length) : line;
    return line.length > 0 ? `${QUOTE_PREFIX}${line}` : line;
  });
  const nextBlock = nextLines.join("\n");

  return {
    text: text.slice(0, blockStart) + nextBlock + text.slice(blockEnd),
    selection: { start: blockStart, end: blockStart + nextBlock.length },
  };
}
