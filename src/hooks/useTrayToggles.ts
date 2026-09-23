import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { InputHelperStatus } from "../lib/inputHelper";
import { expandReady, prepareInputHelperForExpand } from "../lib/inputHelperClient";
import type { Preferences } from "../types/preferences";

/** Tray menu toggles (see src-tauri/src/tray.rs PIN_EVENT / EXPAND_EVENT). */
export function useTrayToggles(options: {
  pinned: boolean;
  expandAsYouType: boolean;
  setPinned: (value: boolean) => void;
  updatePrefs: (patch: Partial<Preferences>) => void;
  onInputStatus: (status: InputHelperStatus) => void;
}) {
  const latest = useRef(options);
  latest.current = options;

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let cancelled = false;
    const subscribe = (event: string, handler: () => void) => {
      void listen(event, handler).then((unlisten) => {
        if (cancelled) unlisten();
        else unlisteners.push(unlisten);
      });
    };
    subscribe("tray-pin-toggle", () => {
      latest.current.setPinned(!latest.current.pinned);
    });
    subscribe("tray-expand-toggle", () => {
      const { expandAsYouType, updatePrefs, onInputStatus } = latest.current;
      if (expandAsYouType) {
        updatePrefs({ expandAsYouType: false });
        return;
      }
      // Same consent path as Settings: Grant keyboard access first if needed.
      void prepareInputHelperForExpand()
        .then((status) => {
          onInputStatus(status);
          if (expandReady(status)) updatePrefs({ expandAsYouType: true });
        })
        .catch(() => undefined);
    });
    return () => {
      cancelled = true;
      for (const unlisten of unlisteners) unlisten();
    };
  }, []);
}
