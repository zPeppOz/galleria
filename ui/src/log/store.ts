import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { logsGet, type CommandError } from "../daemon/commands";
import type { LogEntry } from "../daemon/types";

/** P11: the log keeps in the page at most as many lines as the buffer of the service holds. The newest stay. */
export const MAX_LINES = 2000;
/** What the service serves in one call. */
const PAGE = 1000;
/** How long the lines that arrive in a burst wait to be drawn together: a session that talks a lot does not redraw the list for each. */
const FLUSH_MS = 40;

/**
 * Puts lines into the list: by their number, each once, oldest first, at most `MAX_LINES`. A line whose number is already there is
 * dropped (the page that was read and the event of the same line both come). The usual case, lines newer than all that is held, costs
 * no sort.
 */
export function mergeLines(current: LogEntry[], incoming: LogEntry[]): LogEntry[] {
  if (incoming.length === 0) return current;
  const newest = current.length > 0 ? current[current.length - 1].seq : 0;
  const ascending = incoming.every((entry, at) => (at === 0 ? entry.seq > newest : entry.seq > incoming[at - 1].seq));
  let merged: LogEntry[];
  if (ascending) {
    merged = [...current, ...incoming];
  } else {
    const known = new Set(current.map((entry) => entry.seq));
    const fresh: LogEntry[] = [];
    for (const entry of incoming) {
      if (!known.has(entry.seq)) {
        known.add(entry.seq);
        fresh.push(entry);
      }
    }
    if (fresh.length === 0) return current;
    merged = [...current, ...fresh].sort((a, b) => a.seq - b.seq);
  }
  return merged.length > MAX_LINES ? merged.slice(merged.length - MAX_LINES) : merged;
}

const capital = (word: string): string => word.charAt(0).toUpperCase() + word.slice(1);

/** A line as the file and the clipboard have it: `ts Level Source: message`, the format of `galleria logs` (P12). */
export const formatLine = (entry: LogEntry): string => `${entry.ts} ${capital(entry.level)} ${capital(entry.source)}: ${entry.message}`;

/**
 * The lines of the session: the pages of the service when the tab opens, then the events. `enabled` is false for a profile with no
 * session to show (nothing is asked then); `session` is the number of the session, and a new one starts the list again (the service
 * empties its buffer when a session starts). The lines wait a moment before they are drawn, so that a burst is one drawing.
 */
export function useLog(enabled: boolean, session: number | null): { lines: LogEntry[]; error: CommandError | null } {
  const [lines, setLines] = useState<LogEntry[]>([]);
  const [error, setError] = useState<CommandError | null>(null);
  useEffect(() => {
    setLines([]);
    setError(null);
    if (!enabled) return undefined;
    let closed = false;
    let unlisten: UnlistenFn | null = null;
    let waiting: LogEntry[] = [];
    let timer: ReturnType<typeof setTimeout> | undefined;
    const flush = () => {
      timer = undefined;
      const batch = waiting;
      waiting = [];
      if (!closed && batch.length > 0) setLines((now) => mergeLines(now, batch));
    };
    const later = (entries: LogEntry[]) => {
      waiting.push(...entries);
      if (timer === undefined) timer = setTimeout(flush, FLUSH_MS);
    };
    // no backend behind the page (a test, a browser): no live lines, and nothing is raised
    listen<LogEntry>("galleria:log", (event) => later([event.payload])).then(
      (fn) => {
        if (closed) fn();
        else unlisten = fn;
      },
      () => undefined,
    );
    void (async () => {
      let after: number | null = null;
      try {
        for (;;) {
          const page = await logsGet(after, PAGE);
          if (closed) return;
          later(page.entries);
          if (page.entries.length < PAGE) return;
          after = page.last;
        }
      } catch (failure) {
        if (!closed) setError(failure as CommandError);
      }
    })();
    return () => {
      closed = true;
      if (timer !== undefined) clearTimeout(timer);
      unlisten?.();
    };
  }, [enabled, session]);
  return { lines, error };
}
