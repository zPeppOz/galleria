import { invoke } from "@tauri-apps/api/core";

/**
 * Tells the screen reader an outcome (Q27, spec §12.1). The live regions of the webview stay silent under Orca, so the
 * Rust side emits the ATK signal that Orca does say. This is the only file that invokes the command, and the toast the
 * only caller: nothing else in the webview announces. A refusal is swallowed, because a failed announcement must not
 * break the action that caused it.
 */
export function announce(text: string, assertive: boolean): void {
  try {
    invoke("announce", { text, assertive }).catch(() => undefined);
  } catch {
    // no backend behind the page (a test, a browser): nothing to tell
  }
}
