/** Require Ctrl/Alt/Meta for letter/digit keys — Shift alone is not enough. */
export function formatHotkey(event: {
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}): string | null {
  if (["Shift", "Control", "Alt", "Meta"].includes(event.key)) {
    return null;
  }

  const isLetterOrDigit = event.key.length === 1 && /[A-Za-z0-9]/.test(event.key);
  const hasStrongModifier = event.ctrlKey || event.altKey || event.metaKey;

  if (isLetterOrDigit && !hasStrongModifier) {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  if (event.metaKey) parts.push("Meta");

  let key = event.key;
  if (key === " ") key = "Space";
  else if (key === "ArrowUp") key = "Up";
  else if (key === "ArrowDown") key = "Down";
  else if (key === "ArrowLeft") key = "Left";
  else if (key === "ArrowRight") key = "Right";
  else if (key === "Escape") key = "Esc";
  else if (key.length === 1) key = key.toUpperCase();

  parts.push(key);
  return parts.join("+");
}
