import type { AppInfo, DaemonView } from "../daemon/types";
import { useT } from "../i18n";
import "./StatusBar.css";

/**
 * The bar at the bottom: on the left the state of the VPN as the service reports it, on the right the link to the service
 * and the version of the app. A polite live region, as in B1.
 */
export function StatusBar({ view, info }: { view: DaemonView | null; info: AppInfo | null }) {
  const t = useT();
  const link = view?.link ?? "connecting";
  const state = view?.status?.state;
  return (
    <footer className="status-bar" aria-live="polite">
      <span className="status-bar-state">{state ? t(`state.${state}`) : t("bar.no_state")}</span>
      <span className="status-bar-link">{link === "connected" ? t("bar.connected", { version: view?.daemon_version ?? "?" }) : t(`bar.${link}`)}</span>
      {info ? <span className="status-bar-app">{t("bar.app", { version: info.version })}</span> : null}
    </footer>
  );
}
