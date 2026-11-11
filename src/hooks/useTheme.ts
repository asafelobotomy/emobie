import { useEffect } from "react";
import type { ThemeMode } from "../types/preferences";
import { resolveTheme } from "../lib/uqTheme";
import { VARIANT_ATTRIBUTE } from "../vendor/uq-tokens/uq-tokens.js";

export function useTheme(theme: ThemeMode) {
  useEffect(() => {
    const root = document.documentElement;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      root.dataset.themePref = theme;
      root.dataset.theme = resolveTheme(
        theme,
        media.matches,
        root.getAttribute(VARIANT_ATTRIBUTE),
      );
    };

    apply();

    if (theme !== "system") return;

    const listener = () => apply();
    media.addEventListener("change", listener);
    // uQ's variant arrives (and changes) through src/hooks/useUqTokens.ts.
    const observer = new MutationObserver(listener);
    observer.observe(root, { attributes: true, attributeFilter: [VARIANT_ATTRIBUTE] });
    return () => {
      media.removeEventListener("change", listener);
      observer.disconnect();
    };
  }, [theme]);
}
