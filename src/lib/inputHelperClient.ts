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
 * Runs Grant when listen fails, inject fails (Wayland needs /dev/uinput), or
 * permanent group/udev config is missing (ACL-only / orphaned-GID must not skip Polkit).
 *
 * Unused while as-you-type text expansion is deferred — see
 * prepareInputHelperForPaste for the paste-only path currently in use.
 */
export async function prepareInputHelperForExpand(): Promise<InputHelperStatus> {
  let status = await ensureInputHelperStarted();
  const needsGrant =
    !status.canListen ||
    !status.canInject ||
    status.accessConfigured === false;
  if (needsGrant) {
    status = await runInputHelperAccessSetup();
  }
  return status;
}

/**
 * Ensure helper is running with inject access for "Auto-paste on copy",
 * granting access when needed. Unlike prepareInputHelperForExpand, this does
 * not care about listen capability — as-you-type expansion is deferred, so
 * only the paste-injection half of Grant matters here.
 */
export async function prepareInputHelperForPaste(): Promise<InputHelperStatus> {
  let status = await ensureInputHelperStarted();
  const needsGrant = !status.canInject || status.accessConfigured === false;
  if (needsGrant) {
    status = await runInputHelperAccessSetup();
  }
  return status;
}
