import { describe, expect, it, vi } from "vitest";
import { applyTheme, resolveTheme, watchSystemTheme } from "./theme";

describe("theme", () => {
  it("system_choice_follows_the_system_and_defaults_to_dark", () => {
    expect(resolveTheme("system", false)).toBe("dark");
    expect(resolveTheme("system", true)).toBe("light");
  });

  it("explicit_choice_wins_over_the_system", () => {
    expect(resolveTheme("light", false)).toBe("light");
    expect(resolveTheme("dark", true)).toBe("dark");
  });

  it("apply_sets_data_theme_on_the_root", () => {
    const root = document.createElement("html");
    applyTheme(root, "light");
    expect(root.dataset.theme).toBe("light");
    applyTheme(root, "dark");
    expect(root.dataset.theme).toBe("dark");
  });

  it("watch_reports_changes_and_stops_when_asked", () => {
    let listener: ((e: MediaQueryListEvent) => void) | undefined;
    const add = vi.fn((_: string, fn: (e: MediaQueryListEvent) => void) => { listener = fn; });
    const remove = vi.fn();
    vi.stubGlobal("matchMedia", () => ({ matches: false, addEventListener: add, removeEventListener: remove }));
    try {
      const seen: boolean[] = [];
      const stop = watchSystemTheme((light) => seen.push(light));
      listener?.({ matches: true } as MediaQueryListEvent);
      listener?.({ matches: false } as MediaQueryListEvent);
      expect(seen).toEqual([true, false]);
      stop();
      expect(remove).toHaveBeenCalledWith("change", listener);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
