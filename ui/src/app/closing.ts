import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

/** What was asked when the changes that are not saved stopped it: hide the window in the tray (the «X»), or end the app. */
export type CloseIntent = "hide" | "quit";

const isIntent = (value: unknown): value is CloseIntent => value === "hide" || value === "quit";

/** The last value Rust was told, so that only a change is told (Rust keeps one bit, not the edits). */
let reported = false;

/** Tells Rust whether the page holds changes that are not saved: the bit, nothing of what changed (Q35). Only when it changes. */
export function reportUnsaved(dirty: boolean): void {
  if (reported === dirty) return;
  reported = dirty;
  try {
    invoke("unsaved_set", { dirty }).catch(() => undefined);
  } catch {
    // no backend behind the page (a test, a browser): nobody to tell
  }
}

/**
 * Keeps Rust's bit up to date: it is told when `dirty` changes value, not at every key, and it goes down with the form that raised it (the
 * profile that is edited changes, the form goes away). The only file that invokes `unsaved_set`.
 */
export function useReportUnsaved(dirty: boolean): void {
  useEffect(() => {
    reportUnsaved(dirty);
  }, [dirty]);
  useEffect(() => () => reportUnsaved(false), []);
}

/**
 * Does what a close or a quit asked for, after the person said «Save» (and it worked) or «Discard»: the bit goes down and Rust is asked again, by the
 * same decision. `false` when Rust stopped it again.
 */
export async function carryOut(intent: CloseIntent): Promise<boolean> {
  reportUnsaved(false);
  try {
    const answer = intent === "hide" ? await invoke<{ done: boolean }>("window_hide") : await invoke<{ done: boolean }>("app_quit");
    return answer?.done === true;
  } catch {
    return false;
  }
}

/** The menu of the app and `Ctrl+Q` ask to quit: Rust decides, and says `galleria:close-blocked` when the changes stop it. */
export function quitApp(): void {
  try {
    invoke("app_quit").catch(() => undefined);
  } catch {
    // no backend behind the page: nothing to quit
  }
}

/**
 * Hears that a close or a quit was stopped by changes that are not saved (Rust shows the window first, P8): `onBlocked` opens the question «Save,
 * Discard, Cancel» that the form already has. The only file that listens to `galleria:close-blocked`.
 */
export function useCloseBlocked(onBlocked: (intent: CloseIntent) => void): void {
  const latest = useRef(onBlocked);
  latest.current = onBlocked;
  useEffect(() => {
    let closed = false;
    let unlisten: UnlistenFn | null = null;
    listen<{ intent?: unknown }>("galleria:close-blocked", (event) => {
      if (isIntent(event.payload?.intent)) latest.current(event.payload.intent);
    }).then(
      (fn) => {
        if (closed) fn();
        else unlisten = fn;
      },
      () => undefined,
    );
    return () => {
      closed = true;
      unlisten?.();
    };
  }, []);
}
