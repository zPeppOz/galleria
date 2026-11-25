import type { ReactNode } from "react";
import type { DaemonView } from "../daemon/types";
import { errorText, hasKey, useT } from "../i18n";

const PROTOCOL_NAME = { wireguard: "WireGuard", openvpn: "OpenVPN" } as const;

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </>
  );
}

/** What the daemon says, read only. The connect controls arrive in B5. */
export function StatusView({ view }: { view: DaemonView }) {
  const t = useT();
  const status = view.status;
  if (!status) return null;
  const warnings = view.backends.filter((b) => !b.available);
  const net = status.net;
  return (
    <section className="panel" aria-labelledby="status-title">
      <h2 id="status-title">{t("status.heading")}</h2>
      <p className={`state state-${status.state}`}>{t(`state.${status.state}`)}</p>
      <dl className="facts">
        {status.profile && <Row label={t("status.profile")}>{status.profile.name}</Row>}
        {status.protocol && <Row label={t("status.protocol")}>{PROTOCOL_NAME[status.protocol]}</Row>}
        {status.local_addresses.length > 0 && <Row label={t("status.addresses")}>{status.local_addresses.join(", ")}</Row>}
        {status.remote && <Row label={t("status.remote")}>{status.remote}</Row>}
        {net && <Row label={t("status.interface")}>{net.interface}</Row>}
        {net && net.dns_servers.length > 0 && <Row label={t("status.dns")}>{net.dns_servers.join(", ")}</Row>}
        {net && net.routes.length > 0 && <Row label={t("status.routes")}>{net.routes.join(", ")}</Row>}
      </dl>
      {status.error && (
        <p role="alert" className="error">
          {t("status.error", { code: status.error.code, message: errorText(t, status.error.code, status.error.message) })}
        </p>
      )}
      {warnings.length > 0 && (
        <div>
          <h3>{t("status.warnings")}</h3>
          <ul className="warnings">
            {warnings.map((b) => {
              const key = `backend.${b.unavailable_reason ?? ""}`;
              return (
                <li key={b.protocol}>
                  {hasKey(key) ? t(key) : t("backend.unavailable", { protocol: PROTOCOL_NAME[b.protocol], reason: b.unavailable_reason ?? "?" })}
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </section>
  );
}
