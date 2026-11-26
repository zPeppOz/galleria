import type { AppInfo, DaemonView } from "../daemon/types";
import { InlineAlert } from "../design/InlineAlert";
import { useT } from "../i18n";
import { StatusPanel } from "../connection/StatusPanel";
import { FirstRun } from "../screens/FirstRun";
import { BackendWarnings } from "../screens/BackendWarnings";
import "./ServiceArea.css";

/**
 * What the service has to say above the profile (Q26). Without a link it is the first-run panel; with a link it is the
 * warning about the backends it cannot use and, once a session exists, its status. The list below stays usable always.
 */
export function ServiceArea({ view, info, onRetry, savedCredentials = false }: { view: DaemonView | null; info: AppInfo | null; onRetry: () => void; savedCredentials?: boolean }) {
  const t = useT();
  let content = null;
  if (!view || !info) {
    content = (
      <section className="service-area-connecting" aria-labelledby="connecting-title">
        <h2 id="connecting-title" className="service-area-title">{t("first.connecting.title")}</h2>
        <InlineAlert tone="info" toneLabel={t("alert.info")}>{t("first.connecting.body")}</InlineAlert>
      </section>
    );
  } else if (view.link !== "connected") {
    content = <FirstRun view={view} info={info} onRetry={onRetry} />;
  } else {
    content = (
      <>
        {view.status && view.status.state !== "disconnected" ? <StatusPanel status={view.status} savedCredentials={savedCredentials} /> : null}
        <BackendWarnings view={view} />
      </>
    );
  }
  return content ? <div className="service-area">{content}</div> : null;
}
