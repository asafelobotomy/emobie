import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * When the user enables multiple instances, release the single-instance lock
 * immediately so a second process can start without restarting first.
 * Re-enforcing single-instance requires a restart (handled on next cold start).
 */
export function useAllowMultipleInstances(allow: boolean, ready: boolean) {
  useEffect(() => {
    if (!ready || !allow) return;

    void invoke("release_single_instance_lock").catch((error) => {
      console.error("Failed to release single-instance lock", error);
    });
  }, [allow, ready]);
}
