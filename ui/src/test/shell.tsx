import { render } from "@testing-library/react";
import { useRef, useState } from "react";
import type { SortOrder } from "../app/uiState";
import type { DaemonView } from "../daemon/types";
import type { Announce } from "../design/Toast";
import { Preferences } from "../prefs";
import { arrange } from "../rubrica/arrange";
import type { Rubrica } from "../rubrica/store";
import type { CmdError, GroupActions, ProfileActions, ProfileDetail, ProfilePatch, ProfileSummary, RawCheck, RubricaView } from "../rubrica/types";
import { Shell } from "../shell/Shell";
import { applyPatch, makeDetail, makeInfo, makeRubricaView, makeView } from "./fixtures";

interface Options {
  lang?: "en" | "it";
  onRetry?: () => void;
  announce?: Announce;
  /** The rubrica the shell shows; null is a read that has not answered yet. */
  rubrica?: RubricaView | null;
  error?: CmdError | null;
  /** The saved sort order the window starts with. */
  sort?: SortOrder;
  /** The answer to a write of the window; by default the detail with the patch on it. A test makes it reject to see a refusal. */
  update?: (id: string, patch: ProfilePatch, expectedModified: string) => Promise<ProfileDetail>;
  /** The answer of the check of the raw text; by default the text is clean. */
  validateRaw?: (id: string, text: string) => Promise<RawCheck>;
  /** The group commands; by default they work on the list and the profiles the harness keeps, as the backend would. */
  groups?: GroupActions;
  /** Duplicate, export and delete; by default a copy joins the list, a deleted profile leaves it and the selection moves, as in the app. */
  actions?: ProfileActions;
}

/** The shell with a rubrica that answers at once and keeps its own selection and sort order: no app behind it, nothing to wait for. */
function Harness({ view, options }: { view: DaemonView | null; options: Options }) {
  const first = options.rubrica === undefined ? makeRubricaView() : options.rubrica;
  const [kept, setKept] = useState(first);
  const list = first === null ? null : kept;
  const [sort, setSort] = useState<SortOrder>(options.sort ?? "name");
  const firstId = list ? arrange({ profiles: list.profiles, groups: list.groups, query: "", sort }).groups.flatMap((g) => g.profiles)[0]?.id ?? null : null;
  const [selectedId, select] = useState<string | null>(firstId);
  // what the backend does with the groups: the list changes, and so does the group of the profiles that were in one
  const change = (groups: string[], moved?: { from: string; to: string | null }): string[] => {
    setKept((now) => (now ? { ...now, groups, profiles: moved ? now.profiles.map((p) => (p.group === moved.from ? { ...p, group: moved.to, modified: `${p.modified}+` } : p)) : now.profiles } : now));
    return groups;
  };
  const groupActions: GroupActions = {
    create: async (name) => change([...(list?.groups ?? []), name.trim()]),
    rename: async (from, to) => change((list?.groups ?? []).map((g) => (g === from ? to.trim() : g)), { from, to: to.trim() }),
    remove: async (name) => change((list?.groups ?? []).filter((g) => g !== name), { from: name, to: null }),
    reorder: async (names) => change(names),
  };
  const selected = list?.profiles.find((p) => p.id === selectedId) ?? null;
  // what the backend does with a profile: a copy joins the list, a deleted one leaves it and the selection takes its place
  const copies = useRef(0);
  const order = (profiles: ProfileSummary[]) => arrange({ profiles, groups: list?.groups ?? [], query: "", sort }).groups.flatMap((g) => g.profiles.map((p) => p.id));
  const profileActions: ProfileActions = {
    duplicate: async (id, name) => {
      const source = list?.profiles.find((p) => p.id === id);
      if (!source) throw new Error("no such profile");
      copies.current += 1;
      const copy: ProfileSummary = { ...source, id: `${id}-copy-${copies.current}`, name, favorite: false, modified: `${source.modified}+copy` };
      setKept((now) => (now ? { ...now, profiles: [...now.profiles, copy] } : now));
      return copy;
    },
    export: async () => true,
    remove: async (id) => {
      const before = order(list?.profiles ?? []);
      const rest = (list?.profiles ?? []).filter((p) => p.id !== id);
      const ids = order(rest);
      setKept((now) => (now ? { ...now, profiles: rest } : now));
      select((current) => (current !== id ? current : ids.length === 0 ? null : ids[Math.min(before.indexOf(id), ids.length - 1)]));
    },
  };
  const rubrica: Rubrica = {
    loaded: list !== null || Boolean(options.error),
    view: list,
    error: options.error ?? null,
    selectedId,
    select,
    detail: selected ? makeDetail(selected) : null,
    detailError: null,
    reload: async () => undefined,
    update: options.update ?? (async (id, patch) => applyPatch(makeDetail(list?.profiles.find((p) => p.id === id) ?? selected!), patch)),
    validateRaw: options.validateRaw ?? (async () => ({ issues: [], can_save: true })),
    actions: options.actions ?? profileActions,
    groups: options.groups ?? groupActions,
  };
  return (
    <Shell view={view} info={view ? makeInfo() : null} onRetry={options.onRetry ?? (() => undefined)} announce={options.announce} rubrica={rubrica} sort={sort} onSortChange={setSort} />
  );
}

export const renderShell = (view: DaemonView | null = makeView(), options: Options = {}) =>
  render(
    <Preferences lang={options.lang ?? "en"}>
      <Harness view={view} options={options} />
    </Preferences>,
  );
