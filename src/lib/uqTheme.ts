import type { ThemeMode } from "../types/preferences";

/**
 * The light/dark theme to show. "System" follows uQ's theme variant when uQ's
 * tokens are applied (data-uq-variant on <html>), else the OS preference; an
 * explicit Light or Dark always wins.
 */
export function resolveTheme(
  theme: ThemeMode,
  prefersDark: boolean,
  uqVariant: string | null,
): "light" | "dark" {
  if (theme !== "system") return theme;
  if (uqVariant === "dark" || uqVariant === "light") return uqVariant;
  return prefersDark ? "dark" : "light";
}
