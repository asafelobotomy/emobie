import { useCallback, useEffect, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

export function useCopyEmoji(onCopied: (emoji: string) => void) {
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

  const copyEmoji = useCallback(
    async (emoji: string) => {
      try {
        await writeText(emoji);
      } catch (error) {
        console.error("Failed to copy emoji", error);
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

      onCopied(emoji);
      setCopyError(null);
      setLastCopied(emoji);
      setFlashKey(emoji);

      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
      }
      timerRef.current = window.setTimeout(() => {
        setFlashKey(null);
        setLastCopied(null);
      }, 1200);
    },
    [onCopied],
  );

  return { copyEmoji, lastCopied, flashKey, copyError };
}
