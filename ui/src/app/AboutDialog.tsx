import type { AppInfo } from "../daemon/types";
import { Dialog } from "../design/Dialog";
import { useT } from "../i18n";
import { useUpdate } from "../update/store";
import "../design/Facts.css";

/** «About Galleria»: the app version and the socket from `app_info`, the service version from `daemon_view`, the licence. */
export function AboutDialog(props: { open: boolean; onClose: () => void; info: AppInfo | null; daemonVersion: string | null }) {
  const t = useT();
  const { open, onClose, info, daemonVersion } = props;
  const unknown = "—";
  const { view } = useUpdate();
  // what the check knows: the version that is newer, that there is none, or that the person turned the check off
  const latest = view === null ? unknown : !view.enabled ? t("about.latest.off") : (view.latest ?? t("about.latest.none"));
  return (
    <Dialog open={open} onClose={onClose} title={t("about.title")} description={t("about.description")} closeLabel={t("dialog.close")}>
      <dl className="gx-facts">
        <dt>{t("about.version")}</dt>
        <dd>{info?.version ?? unknown}</dd>
        <dt>{t("about.daemon")}</dt>
        <dd>{daemonVersion ?? t("about.daemon.unknown")}</dd>
        <dt>{t("about.socket")}</dt>
        <dd>{info?.socket_path ?? unknown}</dd>
        <dt>{t("about.latest")}</dt>
        <dd>{latest}</dd>
        <dt>{t("about.licence")}</dt>
        <dd>GPL-3.0-or-later</dd>
      </dl>
    </Dialog>
  );
}
