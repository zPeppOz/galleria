import type { ReactNode } from "react";
import { PROTOCOL_NAME } from "../daemon/protocol";
import type { DaemonView } from "../daemon/types";
import { InlineAlert } from "../design/InlineAlert";
import { errorText, hasKey, useT } from "../i18n";
import "./StatusView.css";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </>
  );
}

/** The VPN backends the service cannot use (OpenVPN missing or too old…), as one warning; nothing when all are available. */
export function BackendWarnings({ view }: { view: DaemonView }) {
  const t = useT();
  const warnings = view.backends.filter((b) => !b.available);
  if (warnings.length === 0) return null;
  return (
    <InlineAlert tone="warn" toneLabel={t("alert.warn")} title={t("status.warnings")}>
      <ul>
        {warnings.map((b) => {
          const key = `backend.${b.unavailable_reason ?? ""}`;
          return (
            <li key={b.protocol}>
              {hasKey(key) ? t(key) : t("backend.unavailable", { protocol: PROTOCOL_NAME[b.protocol], reason: b.unavailable_reason ?? "?" })}
            </li>
          );
        })}
      </ul>
    </InlineAlert>
  );
}

/** What the daemon says, read only. The connect controls arrive in B5. */
export function StatusView({ view }: { view: DaemonView }) {
  const t = useT();
  const status = view.status;
  if (!status) return null;
  const net = status.net;
  return (
    <section className="status-view" aria-labelledby="status-title">
      <h2 id="status-title" className="status-view-title">{t("status.heading")}</h2>
      <p className={`status-view-state status-view-state-${status.state}`}>{t(`state.${status.state}`)}</p>
      <dl className="status-view-facts">
        {status.profile && <Row label={t("status.profile")}>{status.profile.name}</Row>}
        {status.protocol && <Row label={t("status.protocol")}>{PROTOCOL_NAME[status.protocol]}</Row>}
        {status.local_addresses.length > 0 && <Row label={t("status.addresses")}>{status.local_addresses.join(", ")}</Row>}
        {status.remote && <Row label={t("status.remote")}>{status.remote}</Row>}
        {net && <Row label={t("status.interface")}>{net.interface}</Row>}
        {net && net.dns_servers.length > 0 && <Row label={t("status.dns")}>{net.dns_servers.join(", ")}</Row>}
        {net && net.routes.length > 0 && <Row label={t("status.routes")}>{net.routes.join(", ")}</Row>}
      </dl>
      {status.error && (
        <InlineAlert tone="error" toneLabel={t("alert.error")}>
          {t("status.error", { code: status.error.code, message: errorText(t, status.error.code, status.error.message) })}
        </InlineAlert>
      )}
      <BackendWarnings view={view} />
    </section>
  );
}
