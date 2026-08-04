import { useCallback, useEffect, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

export function useCopyEmoji(onCopied: (emoji: string) => void) {
  const [lastCopied, setLastCopied] = useState<string | null>(null);
  const [flashKey, setFlashKey] = useState<string | null>(null);
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
      await writeText(emoji);
      onCopied(emoji);
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

  return { copyEmoji, lastCopied, flashKey };
}
