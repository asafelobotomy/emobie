import { invoke } from "@tauri-apps/api/core";
import type { InputHelperStatus } from "./inputHelper";

/** Start or probe emobie-inputd (idempotent). */
export function ensureInputHelperStarted(): Promise<InputHelperStatus> {
  return invoke<InputHelperStatus>("input_helper_ensure_started");
}

/** Enable or disable as-you-type listening on the daemon. */
export function setInputHelperEnabled(
  enabled: boolean,
): Promise<InputHelperStatus> {
  return invoke<InputHelperStatus>("input_helper_set_enabled", { enabled });
}

/** One-time Polkit keyboard access setup + helper restart. */
export function runInputHelperAccessSetup(): Promise<InputHelperStatus> {
  return invoke<InputHelperStatus>("input_helper_run_access_setup");
}

/** Add or remove the opt-in keyboard read tier (one Polkit prompt). */
export function setInputHelperKeyboardRead(
  enabled: boolean,
): Promise<InputHelperStatus> {
  return invoke<InputHelperStatus>("input_helper_set_keyboard_read", { enabled });
}

/** Ready for Expand as you type: listening, injecting and permanently set up. */
export function expandReady(status: InputHelperStatus | null): boolean {
  return Boolean(
    status?.daemon &&
      status.canListen &&
      status.canInject &&
      status.accessConfigured !== false &&
      status.keyboardReadConfigured,
  );
}

/**
 * Ensure helper is running with listen + inject for Expand as you type. One
 * Grant (with the keyboard-read tier) covers every missing piece. Does not
 * toggle the preference — useInputHelperSync applies it.
 */
export async function prepareInputHelperForExpand(): Promise<InputHelperStatus> {
  const status = await ensureInputHelperStarted();
  return expandReady(status) ? status : setInputHelperKeyboardRead(true);
}

/**
 * Ensure helper is running with inject access for "Auto-paste on copy",
 * granting access when needed. Unlike prepareInputHelperForExpand, this never
 * asks for keyboard read access.
 */
export async function prepareInputHelperForPaste(): Promise<InputHelperStatus> {
  let status = await ensureInputHelperStarted();
  const needsGrant = !status.canInject || status.accessConfigured === false;
  if (needsGrant) {
    status = await runInputHelperAccessSetup();
  }
  return status;
}
