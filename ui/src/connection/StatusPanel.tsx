import type { ReactNode } from "react";
import { PROTOCOL_NAME } from "../daemon/protocol";
import type { Status } from "../daemon/types";
import { InlineAlert } from "../design/InlineAlert";
import { errorText, hasKey, useT, type Translate } from "../i18n";
import "./StatusPanel.css";
import "../design/Facts.css";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </>
  );
}

/** P10: the `detail` of the service in the language of the window when it is one of the known ones (the states of OpenVPN), as it came otherwise. */
export function detailText(t: Translate, detail: string): string {
  const key = `state.detail.${detail}`;
  return hasKey(key) ? t(key) : detail;
}

/**
 * What the service says about the session, read only (spec §16, point 2): the state in words, what it is doing now, the numbers of
 * the network. The error is the text of the catalogue for its code, and the message of the service for a code the catalogue does not
 * know (D22). It is above the profile whatever profile is chosen; the buttons are in the profile. Rust announces the changes of state:
 * nothing here is a live region.
 */
export function StatusPanel({ status }: { status: Status }) {
  const t = useT();
  const net = status.net;
  return (
    <section className="status-view" aria-labelledby="status-title">
      <h2 id="status-title" className="status-view-title">{t("status.heading")}</h2>
      <p className={`status-view-state status-view-state-${status.state}`}>
        {t(`state.${status.state}`)}
        {status.state === "reconnecting" && status.attempt > 0 ? ` · ${t("status.attempt", { n: status.attempt })}` : ""}
      </p>
      {status.detail && status.state !== "error" ? <p className="status-view-detail">{detailText(t, status.detail)}</p> : null}
      <dl className="gx-facts">
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
    </section>
  );
}
