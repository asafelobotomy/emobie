import { useEffect } from "react";
import type { ThemeMode } from "../types/preferences";

function resolveTheme(theme: ThemeMode): "light" | "dark" {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return theme;
}

export function useTheme(theme: ThemeMode) {
  useEffect(() => {
    const apply = () => {
      document.documentElement.dataset.theme = resolveTheme(theme);
    };

    apply();

    if (theme !== "system") return;

    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const listener = () => apply();
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, [theme]);
}
