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

/**
 * Ensure helper is running with listen + inject, granting access when needed.
 * Does not toggle the expand preference — caller updates prefs; useInputHelperSync
 * applies set_enabled from the pref.
 *
 * Runs Grant when listen fails OR permanent group/udev config is missing
 * (ACL-only / orphaned-GID listen must not skip Polkit).
 */
export async function prepareInputHelperForExpand(): Promise<InputHelperStatus> {
  let status = await ensureInputHelperStarted();
  const needsGrant =
    !status.canListen || status.accessConfigured === false;
  if (needsGrant) {
    status = await runInputHelperAccessSetup();
  }
  return status;
}
