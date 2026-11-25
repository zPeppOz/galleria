import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { arrange, type SortKey } from "./arrange";
import type { CmdError, GroupActions, ProfileDetail, ProfilePatch, RawCheck, RubricaView } from "./types";

export interface Rubrica {
  /** The first answer (a list or an error) has arrived. */
  loaded: boolean;
  /** The last list that was read; it stays on screen when a later read fails. */
  view: RubricaView | null;
  /** Why the last read failed, or null. */
  error: CmdError | null;
  selectedId: string | null;
  select: (id: string) => void;
  /** The detail of the selected profile, once it has arrived (never the one of the previous selection). */
  detail: ProfileDetail | null;
  detailError: CmdError | null;
  /** Reads the list again. The window does it by itself on focus; a write of the window calls it when it is done. */
  reload: () => Promise<void>;
  /**
   * Writes a patch to one profile (the id is explicit, so a write that is still going when the selection moves does
   * not land on the wrong profile) and answers with the new detail. It does not read the list: the writer calls
   * `reload` when it has put the answer where it belongs. A refusal is thrown as the error of the command.
   */
  update: (id: string, patch: ProfilePatch, expectedModified: string) => Promise<ProfileDetail>;
  /**
   * Checks the text the editor holds for an OpenVPN profile: what stops a save, line by line. It writes nothing; the save
   * checks again by itself. The findings carry the names of directives, never the text. A failure is thrown as the error
   * of the command.
   */
  validateRaw: (id: string, text: string) => Promise<RawCheck>;
  /** The groups of the list: created, renamed, deleted, put in another order. Each one reads the rubrica again when it worked. */
  groups: GroupActions;
}

/** The ids in the order the list shows them: what «the next one» means when the selected profile disappears. */
export const displayOrder = (view: RubricaView, sort: SortKey = "name", locale?: string): string[] =>
  arrange({ profiles: view.profiles, groups: view.groups, query: "", sort, locale }).groups.flatMap((group) => group.profiles.map((p) => p.id));

function asError(error: unknown): CmdError {
  if (typeof error === "object" && error !== null && typeof (error as CmdError).code === "string") return error as CmdError;
  return { code: "gui_store_unreadable", message: String(error) };
}

/**
 * The rubrica from the app (spec §6): it is read at start, when the window comes back to the front and after every
 * write of the window itself, so that what the CLI or another window did shows up. It keeps the selection: the same
 * profile after a reload; when it is gone, the one that takes its place in the list.
 */
export function useRubrica(order: (view: RubricaView) => string[] = displayOrder): Rubrica {
  const [loaded, setLoaded] = useState(false);
  const [view, setView] = useState<RubricaView | null>(null);
  const [error, setError] = useState<CmdError | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<ProfileDetail | null>(null);
  const [detailError, setDetailError] = useState<CmdError | null>(null);
  // Only the newest read counts: an older answer that arrives late must not overwrite it.
  const newest = useRef(0);
  const lastOrder = useRef<string[]>([]);
  const orderRef = useRef(order);
  orderRef.current = order;

  const reload = useCallback(async () => {
    const mine = ++newest.current;
    try {
      const next = await invoke<RubricaView>("profiles_list");
      if (mine !== newest.current) return;
      setView(next);
      setError(null);
    } catch (failure) {
      if (mine !== newest.current) return;
      setError(asError(failure));
    }
    setLoaded(true);
  }, []);

  const update = useCallback(async (id: string, patch: ProfilePatch, expectedModified: string): Promise<ProfileDetail> => {
    try {
      return await invoke<ProfileDetail>("profile_update", { id, expectedModified, patch });
    } catch (failure) {
      throw asError(failure);
    }
  }, []);

  const validateRaw = useCallback(async (id: string, text: string): Promise<RawCheck> => {
    try {
      return await invoke<RawCheck>("profile_validate_raw", { id, text });
    } catch (failure) {
      throw asError(failure);
    }
  }, []);

  // A group command answers with the new list of groups; the profiles it moved are read with the list.
  const groups = useMemo<GroupActions>(() => {
    const done = async (call: Promise<string[]>): Promise<string[]> => {
      let next: string[];
      try {
        next = await call;
      } catch (failure) {
        throw asError(failure);
      }
      await reload();
      return next;
    };
    return {
      create: (name) => done(invoke<string[]>("group_create", { name })),
      rename: (from, to) => done(invoke<string[]>("group_rename", { from, to })),
      remove: (name) => done(invoke<string[]>("group_delete", { name })),
      reorder: (names) => done(invoke<string[]>("groups_reorder", { names })),
    };
  }, [reload]);

  useEffect(() => {
    void reload();
    const again = () => void reload();
    const visible = () => {
      if (document.visibilityState === "visible") again();
    };
    window.addEventListener("focus", again);
    document.addEventListener("visibilitychange", visible);
    return () => {
      window.removeEventListener("focus", again);
      document.removeEventListener("visibilitychange", visible);
    };
  }, [reload]);

  useEffect(() => {
    if (!view) return;
    const ids = orderRef.current(view);
    const before = lastOrder.current;
    lastOrder.current = ids;
    setSelectedId((current) => {
      if (current !== null && ids.includes(current)) return current;
      if (ids.length === 0) return null;
      const was = current === null ? -1 : before.indexOf(current);
      return ids[was >= 0 ? Math.min(was, ids.length - 1) : 0];
    });
  }, [view]);

  // The detail is read again when the profile changed on disk (its time of modification is in the list).
  const modified = view?.profiles.find((p) => p.id === selectedId)?.modified ?? null;
  useEffect(() => {
    if (selectedId === null) {
      setDetail(null);
      setDetailError(null);
      return undefined;
    }
    let current = true;
    invoke<ProfileDetail>("profile_get", { id: selectedId }).then(
      (next) => {
        if (!current) return;
        setDetail(next);
        setDetailError(null);
      },
      (failure) => {
        if (!current) return;
        const problem = asError(failure);
        setDetail(null);
        setDetailError(problem);
        // gone since the list was read: read the list again, which moves the selection
        if (problem.code === "gui_profile_not_found") void reload();
      },
    );
    return () => {
      current = false;
    };
  }, [selectedId, modified, reload]);

  return {
    loaded,
    view,
    error,
    selectedId,
    select: setSelectedId,
    detail: detail && detail.id === selectedId ? detail : null,
    detailError,
    reload,
    update,
    validateRaw,
    groups,
  };
}
