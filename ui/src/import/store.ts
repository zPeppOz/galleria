import { invoke } from "@tauri-apps/api/core";
import { useMemo } from "react";
import type { CmdError, ProfileSummary } from "../rubrica/types";
import type { ImportActions, ImportPreview } from "./types";

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
