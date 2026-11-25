import { AppMenu } from "../app/AppMenu";
import type { AppInfo } from "../daemon/types";
import { useT } from "../i18n";
import "./TopBar.css";

/** The bar at the top: the name, the sign that the data are not real, and the menu. */
export function TopBar({ info, daemonVersion }: { info: AppInfo | null; daemonVersion: string | null }) {
  const t = useT();
  return (
    <header className="top-bar">
      <span className="top-bar-brand">{t("app.title")}</span>
      <span className="top-bar-demo">{t("demo.banner")}</span>
      <div className="top-bar-actions">
        <AppMenu info={info} daemonVersion={daemonVersion} />
      </div>
    </header>
  );
}
