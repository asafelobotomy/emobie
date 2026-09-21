/**
 * Message from anything a Tauri `invoke` (or plain JS) can throw. Rust
 * commands returning `Err(String)` reject with a bare *string*, not an
 * `Error`, so `error instanceof Error` alone silently drops the real message.
 */
export function errorMessage(error: unknown, fallback: string): string {
  if (typeof error === "string" && error.trim()) return error;
  if (error instanceof Error && error.message.trim()) return error.message;
  return fallback;
}
