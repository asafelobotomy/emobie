import { useCallback, useEffect, useState } from "react";
import { formatHotkey } from "../lib/formatHotkey";

type HotkeyCaptureProps = {
  value: string;
  error: string | null;
  onChange: (hotkey: string) => void;
};

export { formatHotkey } from "../lib/formatHotkey";

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
      <label className="settings-label" id="hotkey-label">
        Global hotkey
      </label>
      <button
        type="button"
        className={`hotkey-capture${capturing ? " active" : ""}`}
        onClick={startCapture}
        aria-pressed={capturing}
        aria-labelledby="hotkey-label"
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
