import { useEffect, useMemo, useRef, useState } from "react";
import type { SortOrder } from "../app/uiState";
import type { AppInfo, DaemonView } from "../daemon/types";
import { Button } from "../design/Button";
import { ConfirmDialog } from "../design/ConfirmDialog";
import { ToastProvider, type Announce } from "../design/Toast";
import { useLang, useT } from "../i18n";
import { ImportDialog } from "../import/ImportDialog";
import type { ImportActions, Incoming } from "../import/types";
import { arrange } from "../rubrica/arrange";
import type { Rubrica } from "../rubrica/store";
import type { ProfileSummary } from "../rubrica/types";
import { GroupsDialog } from "./GroupsDialog";
import { ProfileDetail, type Guard } from "./ProfileDetail";
import { ProfileList, type ListGroup } from "./ProfileList";
import { RubricaNotices } from "./RubricaNotices";
import { SearchBox } from "./SearchBox";
import type { SecretStoreKind } from "../secrets/store";
import { ServiceArea } from "./ServiceArea";
import "./Shell.css";
import { SortControl } from "./SortControl";
import { StatusBar } from "./StatusBar";
import { TopBar } from "./TopBar";

/**
 * The window: top bar, the list on the left (search, sort order, profiles), the service area, the notices of the rubrica
 * and the profile on the right, the status bar. The profiles are the real ones, from `useRubrica`: the shell reads them
 * from its props and never asks the app or the service for anything itself (the daemon link comes in as `view`). The
 * search text is the shell's own; the sort order is the app's, because the selection needs it too.
 */
export function Shell(props: {
  view: DaemonView | null;
  info: AppInfo | null;
  onRetry: () => void;
  announce?: Announce;
  rubrica: Rubrica;
  /** What the window can do with an import: the preview of a text, saving it, letting it go. */
  importer: ImportActions;
  /** A file that came from outside (dropped on the window, or given to the app): the shell opens the import window on it. */
  incoming?: Incoming | null;
  /** A file is dragged over the window: only a hint to draw, with no path in it. */
  dropHover?: boolean;
  /** Where a write of a secret goes (the keyring or a file), once the app knows; `null` until then. */
  secretStore?: SecretStoreKind | null;
  sort: SortOrder;
  onSortChange: (sort: SortOrder) => void;
}) {
  const { view, info, onRetry, announce, rubrica, importer, incoming = null, dropHover = false, secretStore = null, sort, onSortChange } = props;
  const t = useT();
  const lang = useLang();
  const [query, setQuery] = useState("");
  const [importing, setImporting] = useState(false);
  // A file from outside opens the window on its own, in the same pass that brings it (set while drawing, so that the
  // window opens with the offer and not one pass after it).
  const [offered, setOffered] = useState(0);
  if (incoming && incoming.turn !== offered) {
    setOffered(incoming.turn);
    setImporting(true);
  }
  // A profile was imported: the focus goes to its row once the rubrica has been read again and the selection has moved to it.
  const [imported, setImported] = useState<string | null>(null);
  const list = useRef<{ focus: () => void }>(null);
  const root = useRef<HTMLDivElement>(null);
  // The form of the chosen profile says whether it has edits that are not saved; the list asks before it moves away.
  const guard = useRef<Guard | null>(null);
  const [leaving, setLeaving] = useState<string | null>(null);
  const [managing, setManaging] = useState(false);
  // A profile was deleted: the focus goes to the row that takes its place (or to the text of the empty rubrica) as soon as
  // the rubrica has been read again and the selection has moved.
  const [deleted, setDeleted] = useState<string | null>(null);
  const emptyText = useRef<HTMLParagraphElement>(null);
  const profiles = rubrica.view?.profiles;
  const selected = profiles?.find((p) => p.id === rubrica.selectedId) ?? null;
  useEffect(() => {
    if (deleted === null || !profiles || profiles.some((p) => p.id === deleted)) return;
    if (profiles.length === 0) {
      emptyText.current?.focus();
    } else if (selected) {
      list.current?.focus();
    } else {
      return;
    }
    setDeleted(null);
  }, [deleted, profiles, selected]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey && event.key.toLowerCase() === "o") {
        event.preventDefault();
        setImporting(true);
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);
  useEffect(() => {
    if (imported === null || rubrica.selectedId !== imported) return undefined;
    // After the window of the import has given the focus back to where it was
    const timer = setTimeout(() => {
      list.current?.focus();
      setImported(null);
    }, 0);
    return () => clearTimeout(timer);
  }, [imported, rubrica.selectedId]);
  const choose = (id: string) => {
    if (id !== rubrica.selectedId && guard.current?.dirty) setLeaving(id);
    else rubrica.select(id);
  };
  const leave = async (how: "save" | "discard") => {
    const target = leaving;
    setLeaving(null);
    if (target === null) return;
    if (how === "discard") {
      guard.current?.discard();
      rubrica.select(target);
    } else if (await guard.current?.save()) {
      rubrica.select(target);
    } else {
      // The save was refused: the dialog gave the focus back to the list, and the field with the error takes it again.
      setTimeout(() => guard.current?.focusError(), 0);
    }
  };

  /**
   * The window of the import gives the focus back to what had it. When that was nothing (`Ctrl+O` before any Tab, a file that
   * came from outside while nothing had the focus) it would go to the `body`, and a person with a keyboard or a screen reader
   * would be left nowhere: «Import…», the control that opens the window, takes it then.
   */
  const closeImport = () => {
    setImporting(false);
    setTimeout(() => {
      const now = document.activeElement;
      if (now && now !== document.body) return;
      root.current?.querySelector<HTMLElement>(".top-bar-actions button")?.focus();
    }, 0);
  };

  const wasImported = (profile: ProfileSummary) => {
    // a search that hides the new profile would leave the focus nowhere
    setQuery("");
    setImported(profile.id);
    choose(profile.id);
  };

  // The session of the service shows on the row of its profile, when the profile is in the rubrica: a session that
  // someone else started (a profile that is not here) shows nowhere on the list (D21 is B5).
  const session = view?.status && view.status.profile && view.status.state !== "disconnected" ? { id: view.status.profile.id, state: view.status.state } : null;

  const arranged = useMemo(
    () => (rubrica.view ? arrange({ profiles: rubrica.view.profiles, groups: rubrica.view.groups, query, sort, locale: lang }) : { groups: [], total: 0 }),
    [rubrica.view, query, sort, lang],
  );
  // The titles are translated while drawing, so a change of language shows at once; the groups of the user are not translated.
  const groups = useMemo<ListGroup[]>(
    () =>
      arranged.groups.map((group) => ({
        id: group.title ?? "\u0000none",
        title: group.title ?? t("group.none"),
        profiles: group.profiles.map((p) => ({
          id: p.id,
          name: p.name,
          server: p.server ?? "",
          protocol: p.protocol,
          favourite: p.favorite,
          state: session?.id === p.id ? session.state : undefined,
        })),
      })),
    [arranged, t, session?.id, session?.state],
  );
  return (
    <ToastProvider closeLabel={t("toast.close")} announce={announce}>
      <div className="shell" ref={root}>
        <TopBar info={info} daemonVersion={view?.daemon_version ?? null} onImport={() => setImporting(true)} />
        <div className="shell-body">
          <aside className="shell-aside" aria-label={t("shell.profiles")}>
            {profiles && profiles.length > 0 ? (
              <>
                <div className="shell-tools">
                  <SearchBox value={query} onChange={setQuery} count={arranged.total} onArrowDown={() => list.current?.focus()} />
                  <SortControl value={sort} onChange={onSortChange} />
                </div>
                <ProfileList label={t("list.label")} groups={groups} selectedId={selected?.id ?? ""} onSelect={choose} empty={t("search.no_match")} focusRef={list} />
              </>
            ) : (
              <p className="shell-aside-empty" ref={emptyText} tabIndex={-1}>{rubrica.loaded ? t("list.empty") : t("rubrica.loading")}</p>
            )}
            {rubrica.view ? (
              <div className="shell-aside-foot">
                <Button onClick={() => setManaging(true)}>{t("groups.manage")}</Button>
              </div>
            ) : null}
          </aside>
          <main className="shell-main">
            <ServiceArea view={view} info={info} onRetry={onRetry} />
            <RubricaNotices rubrica={rubrica} onImport={() => setImporting(true)} secretStore={secretStore} />
            {selected ? (
              <ProfileDetail
                profile={selected}
                detail={rubrica.detail}
                detailError={rubrica.detailError}
                groups={rubrica.view?.groups ?? []}
                names={profiles?.map((p) => p.name) ?? []}
                inUse={session?.id === selected.id}
                update={rubrica.update}
                validateRaw={rubrica.validateRaw}
                actions={rubrica.actions}
                onSelect={choose}
                onDeleted={setDeleted}
                reload={rubrica.reload}
                guard={guard}
              />
            ) : null}
          </main>
        </div>
        <StatusBar view={view} info={info} />
        <GroupsDialog open={managing} onClose={() => setManaging(false)} groups={rubrica.view?.groups ?? []} profiles={rubrica.view?.profiles ?? []} actions={rubrica.groups} />
        <ImportDialog open={importing} onClose={closeImport} actions={importer} onImported={wasImported} incoming={incoming} secretStore={secretStore} />
        {dropHover ? (
          <div className="shell-drop" aria-hidden="true">
            <span>{t("import.drop_hover")}</span>
          </div>
        ) : null}
        <ConfirmDialog
          open={leaving !== null}
          title={t("leave.title")}
          body={t("leave.body", { name: selected?.name ?? "" })}
          confirmLabel={t("leave.save")}
          alternateLabel={t("leave.discard")}
          cancelLabel={t("leave.cancel")}
          onConfirm={() => void leave("save")}
          onAlternate={() => void leave("discard")}
          onCancel={() => {
            setLeaving(null);
            setImported(null);
          }}
        />
      </div>
    </ToastProvider>
  );
}
