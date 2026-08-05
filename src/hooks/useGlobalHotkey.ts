import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";

async function toggleVisibility() {
  const window = getCurrentWindow();
  const visible = await window.isVisible();
  if (visible) {
    await window.hide();
  } else {
    await window.unminimize();
    await window.show();
    await window.setFocus();
  }
}

export function useGlobalHotkey(hotkey: string, enabled: boolean) {
  const registeredRef = useRef<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled || !hotkey) return;

    let cancelled = false;

    const setup = async () => {
      if (registeredRef.current) {
        try {
          await unregister(registeredRef.current);
        } catch {
          // ignore unregister failures for stale shortcuts
        }
        registeredRef.current = null;
      }

      try {
        await register(hotkey, async (event) => {
          if (event.state === "Pressed") {
            await toggleVisibility();
          }
        });
        if (!cancelled) {
          registeredRef.current = hotkey;
          setError(null);
        } else {
          await unregister(hotkey);
        }
      } catch (err) {
        console.error("Failed to register hotkey", hotkey, err);
        if (!cancelled) {
          setError("Could not register hotkey — try another shortcut.");
        }
      }
    };

    void setup();

    return () => {
      cancelled = true;
      const current = registeredRef.current;
      if (current) {
        void unregister(current).catch(() => undefined);
        registeredRef.current = null;
      }
    };
  }, [hotkey, enabled]);

  return error;
}
