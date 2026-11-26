import { invoke } from "@tauri-apps/api/core";
import type { ThemeChoice } from "../design/theme";
import type { LangChoice } from "../prefs";

export type SortOrder = "name" | "recent";

/** What the webview keeps in `state.toml` (D32). The window size and the last use are Rust's: they are not here. */
export interface UiPrefs {
  lang: LangChoice;
  theme: ThemeChoice;
  sort: SortOrder;
}

/** The values when nothing is saved, or when the saved ones cannot be had in time. */
export const SYSTEM_PREFS: UiPrefs = { lang: "system", theme: "system", sort: "name" };

/** How long the window waits for `ui_state_get` before it starts with the system values. */
export const READ_LIMIT_MS = 300;

const pick = <T extends string>(value: unknown, allowed: readonly T[], fallback: T): T => (allowed.includes(value as T) ? (value as T) : fallback);

function coerce(raw: unknown): UiPrefs {
  const state = (typeof raw === "object" && raw !== null ? raw : {}) as Record<string, unknown>;
  return {
    lang: pick(state.lang, ["system", "en", "it"], "system"),
    theme: pick(state.theme, ["system", "light", "dark"], "system"),
    sort: pick(state.sort, ["name", "recent"], "name"),
  };
}

/** Reads the saved choices. Never rejects: a failure or a slow answer gives the system values. */
export function loadUiPrefs(limitMs: number = READ_LIMIT_MS): Promise<UiPrefs> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(SYSTEM_PREFS), limitMs);
    invoke<unknown>("ui_state_get").then(
      (raw) => {
        clearTimeout(timer);
        resolve(coerce(raw));
      },
      () => {
        clearTimeout(timer);
        resolve(SYSTEM_PREFS);
      },
    );
  });
}

/** Writes some of the choices. Rejects when the write fails: the caller keeps the choice and says so. */
export async function saveUiPrefs(patch: Partial<UiPrefs>): Promise<void> {
  await invoke("ui_state_set", { patch });
}

/** The two things of `[update]` that the page may write (plan B6, T4): the check on or off, and the version it does not want to hear of again. */
export async function saveUpdatePrefs(patch: { update_check?: boolean; update_dismissed?: string }): Promise<void> {
  await invoke("ui_state_set", { patch });
}
