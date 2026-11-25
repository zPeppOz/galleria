import { useMemo } from "react";
import type { AppInfo, DaemonView } from "../daemon/types";
import { ToastProvider, type Announce } from "../design/Toast";
import { useT } from "../i18n";
import { arrange } from "../rubrica/arrange";
import type { Rubrica } from "../rubrica/store";
import { ProfileDetail } from "./ProfileDetail";
import { ProfileList, type ListGroup } from "./ProfileList";
import { RubricaNotices } from "./RubricaNotices";
import { ServiceArea } from "./ServiceArea";
import "./Shell.css";
import { StatusBar } from "./StatusBar";
import { TopBar } from "./TopBar";

/**
 * The window: top bar, the list on the left, the service area, the notices of the rubrica and the profile on the right,
 * the status bar. The profiles are the real ones, from `useRubrica` (T3): the shell reads them from its props and never
 * asks the app or the service for anything itself (the daemon link comes in as `view`).
 */
export function Shell(props: { view: DaemonView | null; info: AppInfo | null; onRetry: () => void; announce?: Announce; rubrica: Rubrica }) {
  const { view, info, onRetry, announce, rubrica } = props;
  const t = useT();
  const profiles = rubrica.view?.profiles;
  const selected = profiles?.find((p) => p.id === rubrica.selectedId) ?? null;
  // The titles are translated while drawing, so a change of language shows at once; the groups of the user are not translated.
  const groups = useMemo<ListGroup[]>(
    () =>
      rubrica.view
        ? arrange(rubrica.view).groups.map((group) => ({
            id: group.title ?? "\u0000none",
            title: group.title ?? t("group.none"),
            profiles: group.profiles.map((p) => ({ id: p.id, name: p.name, server: p.server ?? "", protocol: p.protocol, favourite: p.favorite })),
          }))
        : [],
    [rubrica.view, t],
  );
  return (
    <ToastProvider closeLabel={t("toast.close")} announce={announce}>
      <div className="shell">
        <TopBar info={info} daemonVersion={view?.daemon_version ?? null} />
        <div className="shell-body">
          <aside className="shell-aside" aria-label={t("shell.profiles")}>
            {profiles && profiles.length > 0 ? (
              <ProfileList label={t("list.label")} groups={groups} selectedId={selected?.id ?? ""} onSelect={rubrica.select} />
            ) : (
              <p className="shell-aside-empty">{rubrica.loaded ? t("list.empty") : t("rubrica.loading")}</p>
            )}
          </aside>
          <main className="shell-main">
            <ServiceArea view={view} info={info} onRetry={onRetry} />
            <RubricaNotices rubrica={rubrica} />
            {selected ? <ProfileDetail profile={selected} detail={rubrica.detail} detailError={rubrica.detailError} /> : null}
          </main>
        </div>
        <StatusBar view={view} info={info} />
      </div>
    </ToastProvider>
  );
}
