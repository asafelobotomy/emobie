import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  readText,
  writeText,
} from "@tauri-apps/plugin-clipboard-manager";
import { invoke } from "@tauri-apps/api/core";

export type CopyTextOptions = {
  /** When true, hide window and ask the input helper to paste. */
  autoPaste?: boolean;
};

async function hideWindowForPaste() {
  try {
    await getCurrentWindow().hide();
  } catch {
    // ignore
  }
}

async function tryAutoPaste(text: string): Promise<string | null> {
  let previous: string | null = null;
  try {
    previous = await readText();
  } catch {
    previous = null;
  }

  try {
    await writeText(text);
  } catch (error) {
    console.error("Failed to write clipboard for paste", error);
    return "Copy failed";
  }

  await hideWindowForPaste();

  try {
    await invoke("input_helper_inject_paste");
  } catch (error) {
    console.error("Auto-paste inject failed", error);
    return "Copied — paste unavailable";
  }

  if (previous !== null && previous !== text) {
    window.setTimeout(() => {
      void writeText(previous).catch(() => undefined);
    }, 400);
  }
  return null;
}

export function useCopyText(onCopied?: (text: string) => void) {
  const [lastCopied, setLastCopied] = useState<string | null>(null);
  const [flashKey, setFlashKey] = useState<string | null>(null);
  const [copyError, setCopyError] = useState<string | null>(null);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
      }
    };
  }, []);

  const copyText = useCallback(
    async (text: string, options: CopyTextOptions = {}) => {
      const finishOk = (statusText: string | null) => {
        onCopied?.(text);
        setCopyError(statusText);
        setLastCopied(text);
        setFlashKey(text);
        if (timerRef.current !== null) {
          window.clearTimeout(timerRef.current);
        }
        timerRef.current = window.setTimeout(() => {
          setFlashKey(null);
          setLastCopied(null);
          if (statusText) setCopyError(null);
        }, 1200);
      };

      if (options.autoPaste) {
        const pasteError = await tryAutoPaste(text);
        if (pasteError === "Copy failed") {
          setCopyError(pasteError);
          setLastCopied(null);
          setFlashKey(null);
          if (timerRef.current !== null) {
            window.clearTimeout(timerRef.current);
          }
          timerRef.current = window.setTimeout(() => {
            setCopyError(null);
          }, 1600);
          return;
        }
        finishOk(pasteError);
        return;
      }

      try {
        await writeText(text);
      } catch (error) {
        console.error("Failed to copy text", error);
        setCopyError("Copy failed");
        setLastCopied(null);
        setFlashKey(null);
        if (timerRef.current !== null) {
          window.clearTimeout(timerRef.current);
        }
        timerRef.current = window.setTimeout(() => {
          setCopyError(null);
        }, 1600);
        return;
      }

      finishOk(null);
    },
    [onCopied],
  );

  return { copyText, lastCopied, flashKey, copyError };
}

/** @deprecated Prefer useCopyText; kept for emoji call sites. */
export function useCopyEmoji(onCopied: (emoji: string) => void) {
  const { copyText, lastCopied, flashKey, copyError } = useCopyText(onCopied);
  const copyEmoji = useCallback(
    (emoji: string, options?: CopyTextOptions) => copyText(emoji, options),
    [copyText],
  );
  return { copyEmoji, lastCopied, flashKey, copyError };
}
