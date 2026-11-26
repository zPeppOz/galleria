import type { ReactNode } from "react";
import type { Protocol } from "../daemon/types";
import { useLang, useT } from "../i18n";
import { exactBytes, formatBytes, formatDuration, formatHandshakeAge } from "./format";
import { useConnection, isLive, useNow, useStats } from "./store";

function Empty({ label }: { label: string }) {
  return (
    <>
      <span aria-hidden="true">—</span>
      <span className="visually-hidden">{label}</span>
    </>
  );
}

/**
 * The strip under the header of a profile: the state of its session, the address, the traffic and the last handshake (WireGuard, when
 * the backend says it has one) or the duration. For a profile with no session the cells are empty. The numbers move with the events
 * of the service and are **not** a live region (Q27, rule 3): a screen reader reads them when asked, and says nothing every second.
 */
export function Stats({ profile }: { profile: { id: string; protocol: Protocol } }) {
  const t = useT();
  const lang = useLang();
  const connection = useConnection();
  const status = connection.status;
  const mine = isLive(status) && status.profile?.id === profile.id ? status : null;
  const connected = mine?.state === "connected";
  const stats = useStats(mine);
  const now = useNow(connected);
  const hasHandshake = connection.backends.find((b) => b.protocol === profile.protocol)?.capabilities.handshake ?? false;
  const empty = t("strip.empty");

  const started = mine?.since ? Date.parse(mine.since) : connection.connectedAt;
  const cells: [string, string, ReactNode][] = [
    [
      "strip.state",
      t("strip.state"),
      mine ? (
        <>
          {t(`state.${mine.state}`)}
          {mine.state === "reconnecting" && mine.attempt > 0 ? ` · ${t("status.attempt", { n: mine.attempt })}` : ""}
        </>
      ) : (
        <Empty label={empty} />
      ),
    ],
    ["strip.ip", t("strip.ip"), mine && mine.local_addresses.length > 0 ? mine.local_addresses.join(", ") : <Empty label={empty} />],
    [
      "strip.traffic",
      t("strip.traffic"),
      connected && stats ? (
        <span title={`${exactBytes(stats.rx_bytes, lang)} / ${exactBytes(stats.tx_bytes, lang)}`}>{t("stats.traffic", { rx: formatBytes(stats.rx_bytes, lang), tx: formatBytes(stats.tx_bytes, lang) })}</span>
      ) : (
        <Empty label={empty} />
      ),
    ],
  ];
  if (hasHandshake && profile.protocol === "wireguard") {
    cells.push(["strip.handshake", t("strip.handshake"), connected && stats?.last_handshake ? formatHandshakeAge(stats.last_handshake, now, lang) : <Empty label={empty} />]);
  } else {
    cells.push(["strip.duration", t("strip.duration"), connected && started !== null && !Number.isNaN(started) ? formatDuration((now - started) / 1000) : <Empty label={empty} />]);
  }
  return (
    <dl className="detail-strip">
      {cells.map(([key, label, value]) => (
        <div key={key} className="detail-strip-cell" data-cell={key}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}
