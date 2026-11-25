import type { AppInfo } from "../daemon/types";
import { Dialog } from "../design/Dialog";
import { useT } from "../i18n";

/** «About Galleria»: the app version and the socket from `app_info`, the service version from `daemon_view`, the licence. */
export function AboutDialog(props: { open: boolean; onClose: () => void; info: AppInfo | null; daemonVersion: string | null }) {
  const t = useT();
  const { open, onClose, info, daemonVersion } = props;
  const unknown = "—";
  return (
    <Dialog open={open} onClose={onClose} title={t("about.title")} description={t("about.description")} closeLabel={t("dialog.close")}>
      <dl className="facts">
        <dt>{t("about.version")}</dt>
        <dd>{info?.version ?? unknown}</dd>
        <dt>{t("about.daemon")}</dt>
        <dd>{daemonVersion ?? t("about.daemon.unknown")}</dd>
        <dt>{t("about.socket")}</dt>
        <dd>{info?.socket_path ?? unknown}</dd>
        <dt>{t("about.licence")}</dt>
        <dd>GPL-3.0-or-later</dd>
      </dl>
    </Dialog>
  );
}
