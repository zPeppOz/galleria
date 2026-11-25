import { useMemo, useState } from "react";
import type { AppInfo, DaemonView } from "../daemon/types";
import { ToastProvider } from "../design/Toast";
import { useT } from "../i18n";
import { DEMO_GROUPS, DEMO_PROFILES } from "./demo-data";
import { ProfileDetail } from "./ProfileDetail";
import { ProfileList, type ListGroup } from "./ProfileList";
import { ServiceArea } from "./ServiceArea";
import "./Shell.css";
import { StatusBar } from "./StatusBar";
import { TopBar } from "./TopBar";

/**
 * The window: top bar, the list on the left, the service area and the profile on the right, the status bar. In B2 the
 * profiles are the sample data of `demo-data.ts`; the shell never asks the service for anything (the daemon link comes
 * in as `view`).
 */
export function Shell({ view, info, onRetry }: { view: DaemonView | null; info: AppInfo | null; onRetry: () => void }) {
  const t = useT();
  const [selectedId, setSelectedId] = useState(DEMO_PROFILES[0].id);
  const profile = DEMO_PROFILES.find((p) => p.id === selectedId) ?? DEMO_PROFILES[0];
  // Titles are translated while drawing, so a change of language shows at once.
  const groups = useMemo<ListGroup[]>(
    () => DEMO_GROUPS.map((group) => ({ id: group, title: t(`group.${group}`), profiles: DEMO_PROFILES.filter((p) => p.group === group) })),
    [t],
  );
  return (
    <ToastProvider closeLabel={t("toast.close")}>
      <div className="shell">
        <TopBar info={info} daemonVersion={view?.daemon_version ?? null} />
        <div className="shell-body">
          <aside className="shell-aside" aria-label={t("shell.profiles")}>
            <ProfileList label={t("list.label")} groups={groups} selectedId={profile.id} onSelect={setSelectedId} />
          </aside>
          <main className="shell-main">
            <ServiceArea view={view} info={info} onRetry={onRetry} />
            <ProfileDetail profile={profile} groupTitle={t(`group.${profile.group}`)} />
          </main>
        </div>
        <StatusBar view={view} info={info} />
      </div>
    </ToastProvider>
  );
}
