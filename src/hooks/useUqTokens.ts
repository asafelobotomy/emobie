import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { watchTokens } from "../vendor/uq-tokens/uq-tokens.js";
import "../vendor/uq-tokens/uq-tokens.css";
import "../styles/uq.css";

/**
 * Follows uQ's theme tokens (src-tauri/src/uq_tokens.rs reads them and emits
 * `uq-tokens-changed`). Without uQ the read returns null and nothing is set.
 */
export function useUqTokens() {
  useEffect(() => {
    const watch = watchTokens({
      read: () => invoke<string | null>("uq_tokens"),
      subscribe: (callback) => listen("uq-tokens-changed", () => callback()),
    });
    return () => watch.stop();
  }, []);
}
