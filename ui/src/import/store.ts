import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import type { CmdError, ProfileSummary } from "../rubrica/types";
import type { ImportActions, ImportOffer, ImportPreview, Incoming } from "./types";

/**
 * The files that come from outside: the one that was waiting when the page started (`import_pending`, asked once, after
 * the page listens) and the ones that arrive while it runs (`galleria:open-file`, a preview or an error; Rust made it and
 * the page never has a path). `hovering` is true while a file is dragged over the window (`galleria:drop-hover`): it is
 * only for drawing. The page listens to no event of Tauri, only to these two of Galleria (`boundary.test.ts`).
 */
export function useIncoming(): { incoming: Incoming | null; hovering: boolean } {
  const [incoming, setIncoming] = useState<Incoming | null>(null);
  const [hovering, setHovering] = useState(false);
  useEffect(() => {
    let closed = false;
    // What arrives while the answer of `import_pending` is on its way is the very offer that answer holds.
    let waiting = true;
    let heard = false;
    const unlisten: UnlistenFn[] = [];
    const keep = (fn: UnlistenFn) => (closed ? fn() : unlisten.push(fn));
    const receive = (offer: ImportOffer) => setIncoming((was) => ({ offer, turn: (was?.turn ?? 0) + 1 }));
    void (async () => {
      try {
        keep(
          await listen<ImportOffer>("galleria:open-file", (event) => {
            if (waiting) heard = true;
            receive(event.payload);
            // It is shown: it does not wait for a page any more.
            if (!waiting) void invoke("import_pending").catch(() => undefined);
          }),
        );
        keep(await listen<{ active: boolean }>("galleria:drop-hover", (event) => setHovering(event.payload.active)));
        // A mount that was undone at once (the first one of a development build) must not take the offer from the next.
        if (closed) return;
        const first = await invoke<ImportOffer | null>("import_pending");
        waiting = false;
        if (first && !heard && !closed) receive(first);
      } catch (failure) {
        console.error("galleria-ui: cannot ask for the file that came from outside", failure);
      }
    })();
    return () => {
      closed = true;
      unlisten.forEach((fn) => fn());
    };
  }, []);
  return { incoming, hovering };
}

function asError(error: unknown): CmdError {
  if (typeof error === "object" && error !== null && typeof (error as CmdError).code === "string") return error as CmdError;
  return { code: "gui_store_unreadable", message: String(error) };
}

/**
 * The commands of the import. The text is the one thing of a profile that goes from the page to Rust (the user typed or
 * pasted it); a file is chosen by Rust, and no command takes a path. `reload` reads the rubrica again after a profile was
 * saved.
 */
export function useImportActions(reload: () => Promise<void>): ImportActions {
  return useMemo<ImportActions>(
    () => ({
      fromText: async (text) => {
        try {
          return await invoke<ImportPreview>("import_from_text", { text });
        } catch (failure) {
          throw asError(failure);
        }
      },
      pickFile: async () => {
        try {
          return await invoke<ImportPreview | null>("import_pick_file");
        } catch (failure) {
          throw asError(failure);
        }
      },
      commit: async (id, name, group) => {
        let saved: ProfileSummary;
        try {
          saved = await invoke<ProfileSummary>("import_commit", { id, name, group });
        } catch (failure) {
          throw asError(failure);
        }
        await reload();
        return saved;
      },
      discard: async (id) => {
        try {
          await invoke("import_discard", { id });
        } catch {
          // a preview that cannot be let go is let go by the clock of Rust
        }
      },
    }),
    [reload],
  );
}
