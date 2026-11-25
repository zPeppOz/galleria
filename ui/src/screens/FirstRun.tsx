import { useRef } from "react";
import { joinGroup, START_SERVICE } from "../daemon/commands";
import type { AppInfo, DaemonView, LinkState } from "../daemon/types";
import { useT } from "../i18n";

interface Props { view: DaemonView; info: AppInfo; onRetry: () => void }

/** The panel for every link state but `connected` (D27). It only shows commands; it never runs them. */
export function FirstRun({ view, info, onRetry }: Props) {
  const t = useT();
  const copyOf = useRef<string>("");
  const link = view.link as Exclude<LinkState, "connected">;
  const command = link === "daemon_not_running" ? START_SERVICE : link === "permission_denied" ? joinGroup(info) : null;
  copyOf.current = command ?? "";
  const range = view.incompatible;
  return (
    <section className="panel" aria-labelledby="first-title">
      <h2 id="first-title">{t(`first.${link}.title`)}</h2>
      {link === "incompatible_protocol" ? (
        <p>{t("first.incompatible_protocol.body", { gui: info.version, daemon: range?.daemon_version ?? "?", min: range?.daemon_min ?? "?", max: range?.daemon_max ?? "?" })}</p>
      ) : (
        <p>{t(`first.${link}.body`)}</p>
      )}
      {command && (
        <div className="cmd-row">
          <code className="cmd">{command}</code>
          <button type="button" className="btn" onClick={() => void navigator.clipboard.writeText(copyOf.current)}>
            {t("first.copy")}
          </button>
        </div>
      )}
      {link === "permission_denied" && <p>{t("first.permission_denied.relogin")}</p>}
      {link !== "connecting" && (
        <button type="button" className="btn primary" onClick={onRetry}>
          {t("first.retry")}
        </button>
      )}
    </section>
  );
}
