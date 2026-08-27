import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import type { Macro } from "../types/preferences";

async function toggleVisibility(pinned: boolean) {
  const window = getCurrentWindow();
  const visible = await window.isVisible();
  if (visible) {
    await window.hide();
  } else {
    await window.unminimize();
    await window.show();
    try {
      await invoke("apply_window_pin", { pinned });
    } catch {
      await window.setAlwaysOnTop(pinned);
    }
    await window.setFocus();
  }
}

export type MacroHotkeyBinding = {
  hotkey: string;
  expansion: string;
};

async function unregisterAll(keys: string[]) {
  for (const key of keys) {
    try {
      await unregister(key);
    } catch {
      // ignore stale shortcuts
    }
  }
}

export function useGlobalHotkeys(options: {
  summonHotkey: string;
  summonEnabled: boolean;
  /** Re-applied when summon shows the window (WMs drop keep-above on hide). */
  pinned?: boolean;
  macros: Macro[];
  onMacroHotkey: (expansion: string) => void | Promise<void>;
  ready: boolean;
}) {
  const registeredRef = useRef<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const onMacroRef = useRef(options.onMacroHotkey);
  onMacroRef.current = options.onMacroHotkey;
  const pinnedRef = useRef(options.pinned ?? false);
  pinnedRef.current = options.pinned ?? false;

  const macroBindings: MacroHotkeyBinding[] = options.macros
    .filter((macro) => macro.enabled && macro.hotkey)
    .map((macro) => ({
      hotkey: macro.hotkey as string,
      expansion: macro.expansion,
    }));

  const bindingKey = macroBindings
    .map((item) => `${item.hotkey}\0${item.expansion}`)
    .join("|");

  useEffect(() => {
    if (!options.ready) return;

    let cancelled = false;

    const setup = async () => {
      await unregisterAll(registeredRef.current);
      registeredRef.current = [];

      const next: string[] = [];
      let firstError: string | null = null;

      if (options.summonEnabled && options.summonHotkey) {
        try {
          await register(options.summonHotkey, async (event) => {
            if (event.state === "Pressed") {
              await toggleVisibility(pinnedRef.current);
            }
          });
          next.push(options.summonHotkey);
        } catch (err) {
          console.error("Failed to register summon hotkey", err);
          firstError = "Could not register hotkey — try another shortcut.";
        }
      }

      const used = new Set(next);
      for (const binding of macroBindings) {
        if (used.has(binding.hotkey)) {
          if (!firstError) {
            firstError = `Macro hotkey ${binding.hotkey} conflicts with another shortcut.`;
          }
          continue;
        }
        try {
          const expansion = binding.expansion;
          await register(binding.hotkey, async (event) => {
            if (event.state === "Pressed") {
              await onMacroRef.current(expansion);
            }
          });
          next.push(binding.hotkey);
          used.add(binding.hotkey);
        } catch (err) {
          console.error("Failed to register macro hotkey", binding.hotkey, err);
          if (!firstError) {
            firstError = `Could not register macro hotkey ${binding.hotkey}.`;
          }
        }
      }

      if (cancelled) {
        await unregisterAll(next);
        return;
      }
      registeredRef.current = next;
      setError(firstError);
    };

    void setup();

    return () => {
      cancelled = true;
      const current = registeredRef.current;
      registeredRef.current = [];
      void unregisterAll(current);
    };
    // bindingKey captures macro hotkey+expansion changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    options.ready,
    options.summonEnabled,
    options.summonHotkey,
    bindingKey,
  ]);

  return error;
}

/** Back-compat wrapper used if anything still imports the old name. */
export function useGlobalHotkey(hotkey: string, enabled: boolean) {
  return useGlobalHotkeys({
    summonHotkey: hotkey,
    summonEnabled: enabled,
    macros: [],
    onMacroHotkey: () => undefined,
    ready: enabled || Boolean(hotkey),
  });
}
