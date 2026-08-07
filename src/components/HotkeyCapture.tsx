import { useCallback, useEffect, useState } from "react";

type HotkeyCaptureProps = {
  value: string;
  error: string | null;
  onChange: (hotkey: string) => void;
};

function isLetterOrDigitKey(key: string): boolean {
  return key.length === 1 && /[A-Za-z0-9]/.test(key);
}

/** Require Ctrl/Alt/Meta for letter/digit keys — Shift alone is not enough. */
export function formatHotkey(event: KeyboardEvent): string | null {
  if (["Shift", "Control", "Alt", "Meta"].includes(event.key)) {
    return null;
  }

  const hasStrongModifier = event.ctrlKey || event.altKey || event.metaKey;

  if (isLetterOrDigitKey(event.key) && !hasStrongModifier) {
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

export function HotkeyCapture({ value, error, onChange }: HotkeyCaptureProps) {
  const [capturing, setCapturing] = useState(false);
  const [draftHotkey, setDraftHotkey] = useState(value);
  const [hotkeyHint, setHotkeyHint] = useState<string | null>(null);

  useEffect(() => {
    setDraftHotkey(value);
  }, [value]);

  useEffect(() => {
    if (!capturing) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      const next = formatHotkey(event);
      if (!next) {
        setHotkeyHint("Letter and number keys need Ctrl, Alt, or Meta");
        return;
      }
      setDraftHotkey(next);
      onChange(next);
      setHotkeyHint(null);
      setCapturing(false);
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capturing, onChange]);

  const startCapture = useCallback(() => {
    setHotkeyHint(null);
    setCapturing(true);
  }, []);

  const cancelCapture = useCallback(() => {
    setCapturing(false);
    setHotkeyHint(null);
  }, []);

  useEffect(() => {
    if (!capturing) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      cancelCapture();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capturing, cancelCapture]);

  return (
    <div className="settings-row">
      <span className="settings-label">Global hotkey</span>
      <button
        type="button"
        className={`hotkey-capture${capturing ? " active" : ""}`}
        onClick={startCapture}
        aria-pressed={capturing}
      >
        {capturing ? "Press a shortcut…" : draftHotkey}
      </button>
      {hotkeyHint ? (
        <p className="settings-hint">{hotkeyHint}</p>
      ) : (
        <p className="settings-hint">
          Letters and numbers need Ctrl, Alt, or Meta (Shift alone is not
          enough). Function keys and punctuation can stand alone.
        </p>
      )}
      {error ? <p className="settings-error">{error}</p> : null}
    </div>
  );
}
