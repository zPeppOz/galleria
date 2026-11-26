import { isLive, useConnection } from "../connection/store";
import type { AppInfo, DaemonView } from "../daemon/types";
import { useT } from "../i18n";
import "./StatusBar.css";

/**
 * The bar at the bottom: on the left the state of the VPN as the service reports it and, next to it, the session («Ufficio Napoli» ·
 * 10.8.0.2); on the right the link to the service and the version of the app. A polite live region, as in B1.
 */
export function StatusBar({ view, info }: { view: DaemonView | null; info: AppInfo | null }) {
  const t = useT();
  const { foreign } = useConnection();
  const link = view?.link ?? "connecting";
  const status = view?.status;
  const state = status?.state;
  const session = isLive(status ?? null) && status?.profile ? status.profile.name : null;
  const address = status?.state === "connected" ? status.local_addresses[0] : undefined;
  return (
    <footer className="status-bar" aria-live="polite">
      <span className="status-bar-state">{state ? t(`state.${state}`) : t("bar.no_state")}</span>
      {foreign?.profile ? (
        <span className="status-bar-session">{t("bar.foreign", { name: foreign.profile.name })}</span>
      ) : session ? (
        <span className="status-bar-session">{address ? t("bar.session_ip", { name: session, ip: address }) : t("bar.session", { name: session })}</span>
      ) : null}
      <span className="status-bar-link">{link === "connected" ? t("bar.connected", { version: view?.daemon_version ?? "?" }) : t(`bar.${link}`)}</span>
      {info ? <span className="status-bar-app">{t("bar.app", { version: info.version })}</span> : null}
    </footer>
  );
}
