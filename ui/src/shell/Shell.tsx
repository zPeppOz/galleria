import { useMemo, useRef, useState } from "react";
import type { SortOrder } from "../app/uiState";
import type { AppInfo, DaemonView } from "../daemon/types";
import { ConfirmDialog } from "../design/ConfirmDialog";
import { ToastProvider, type Announce } from "../design/Toast";
import { useLang, useT } from "../i18n";
import { arrange } from "../rubrica/arrange";
import type { Rubrica } from "../rubrica/store";
import { ProfileDetail, type Guard } from "./ProfileDetail";
import { ProfileList, type ListGroup } from "./ProfileList";
import { RubricaNotices } from "./RubricaNotices";
import { SearchBox } from "./SearchBox";
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
  sort: SortOrder;
  onSortChange: (sort: SortOrder) => void;
}) {
  const { view, info, onRetry, announce, rubrica, sort, onSortChange } = props;
  const t = useT();
  const lang = useLang();
  const [query, setQuery] = useState("");
  const list = useRef<{ focus: () => void }>(null);
  // The form of the chosen profile says whether it has edits that are not saved; the list asks before it moves away.
  const guard = useRef<Guard | null>(null);
  const [leaving, setLeaving] = useState<string | null>(null);
  const profiles = rubrica.view?.profiles;
  const selected = profiles?.find((p) => p.id === rubrica.selectedId) ?? null;
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
      <div className="shell">
        <TopBar info={info} daemonVersion={view?.daemon_version ?? null} />
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
              <p className="shell-aside-empty">{rubrica.loaded ? t("list.empty") : t("rubrica.loading")}</p>
            )}
          </aside>
          <main className="shell-main">
            <ServiceArea view={view} info={info} onRetry={onRetry} />
            <RubricaNotices rubrica={rubrica} />
            {selected ? (
              <ProfileDetail profile={selected} detail={rubrica.detail} detailError={rubrica.detailError} groups={rubrica.view?.groups ?? []} update={rubrica.update} reload={rubrica.reload} guard={guard} />
            ) : null}
          </main>
        </div>
        <StatusBar view={view} info={info} />
        <ConfirmDialog
          open={leaving !== null}
          title={t("leave.title")}
          body={t("leave.body", { name: selected?.name ?? "" })}
          confirmLabel={t("leave.save")}
          alternateLabel={t("leave.discard")}
          cancelLabel={t("leave.cancel")}
          onConfirm={() => void leave("save")}
          onAlternate={() => void leave("discard")}
          onCancel={() => setLeaving(null)}
        />
      </div>
    </ToastProvider>
  );
}
