export type ThemeChoice = "system" | "light" | "dark";
export type Theme = "light" | "dark";

const LIGHT_QUERY = "(prefers-color-scheme: light)";

/** `system` follows the system and is dark unless the system asks for light; an explicit choice always wins. */
export function resolveTheme(choice: ThemeChoice, systemPrefersLight: boolean): Theme {
  if (choice === "system") return systemPrefersLight ? "light" : "dark";
  return choice;
}

/** The theme is always written on the root as `data-theme`, so the CSS has one switch and no media query. */
export function applyTheme(root: HTMLElement, theme: Theme): void {
  root.dataset.theme = theme;
}

/** True when the system asks for a light theme. A webview without `matchMedia` is dark. */
export function systemPrefersLight(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia(LIGHT_QUERY).matches;
}

/** Calls `onChange` when the system theme changes; returns the function that stops listening. */
export function watchSystemTheme(onChange: (prefersLight: boolean) => void): () => void {
  if (typeof window.matchMedia !== "function") return () => {};
  const query = window.matchMedia(LIGHT_QUERY);
  const listener = (event: MediaQueryListEvent) => onChange(event.matches);
  query.addEventListener("change", listener);
  return () => query.removeEventListener("change", listener);
}
