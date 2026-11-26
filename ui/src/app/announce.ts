import { invoke } from "@tauri-apps/api/core";
import { useEffect } from "react";
import { SESSION_ERROR_CODES } from "../daemon/types";
import { useT, type Translate } from "../i18n";

/**
 * Tells the screen reader an outcome (Q27, spec §12.1). The live regions of the webview stay silent under Orca, so the
 * Rust side emits the ATK signal that Orca does say. This is the only file that invokes the commands about announcements:
 * the toast is the only caller of `announce`, and `usePhrasesForRust` the only one of `announce_phrases`. A refusal is
 * swallowed, because a failed announcement must not break the action that caused it.
 */
export function announce(text: string, assertive: boolean): void {
  try {
    invoke("announce", { text, assertive }).catch(() => undefined);
  } catch {
    // no backend behind the page (a test, a browser): nothing to tell
  }
}

/** `{name}` stays as it is in the text: Rust puts the name of the profile there when the state changes (Q43). */
const NAME = { name: "{name}" };

/** The sentences Rust says when the state changes, in the language of `t`. */
export function statePhrases(t: Translate) {
  const error: Record<string, string> = {};
  for (const code of SESSION_ERROR_CODES) error[code] = t("announce.error", { ...NAME, message: t(`error.${code}`) });
  return {
    connected: t("announce.connected", NAME),
    disconnected: t("announce.disconnected", NAME),
    reconnecting: t("announce.reconnecting", NAME),
    error,
    error_other: t("announce.error_other", NAME),
  };
}

/**
 * The changes of state are announced by Rust, from the place that emits the event (Q43, Q27): the page does not call
 * `announce` for them. What Rust needs from the page is the text, in the language the window has now; it is given at the
 * start and again at every change of language.
 */
export function usePhrasesForRust(): void {
  const t = useT();
  useEffect(() => {
    try {
      invoke("announce_phrases", { phrases: statePhrases(t) }).catch(() => undefined);
    } catch {
      // no backend behind the page: nothing to tell
    }
  }, [t]);
}
