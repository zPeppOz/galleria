import { AppMenu } from "../app/AppMenu";
import type { AppInfo } from "../daemon/types";
import { Button } from "../design/Button";
import { useT } from "../i18n";
import "./TopBar.css";

/** The bar at the top: the name, «Import…» and the menu. `Ctrl+O` does what «Import…» does. */
export function TopBar({ info, daemonVersion, onImport }: { info: AppInfo | null; daemonVersion: string | null; onImport: () => void }) {
  const t = useT();
  return (
    <header className="top-bar">
      <span className="top-bar-brand">{t("app.title")}</span>
      <div className="top-bar-actions">
        <Button onClick={onImport}>{t("import.open")}</Button>
        <AppMenu info={info} daemonVersion={daemonVersion} />
      </div>
    </header>
  );
}
