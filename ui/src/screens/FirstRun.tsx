import { useRef } from "react";
import { joinGroup, START_SERVICE } from "../daemon/commands";
import type { AppInfo, DaemonView, LinkState } from "../daemon/types";
import { Button } from "../design/Button";
import { icons } from "../design/icons";
import { InlineAlert } from "../design/InlineAlert";
import { useT } from "../i18n";
import "./FirstRun.css";

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
    <section className="first-run" aria-labelledby="first-title">
      <h2 id="first-title" className="first-run-title">{t(`first.${link}.title`)}</h2>
      {link === "incompatible_protocol" ? (
        <InlineAlert tone="warn" toneLabel={t("alert.warn")}>
          {t("first.incompatible_protocol.body", { gui: info.version, daemon: range?.daemon_version ?? "?", min: range?.daemon_min ?? "?", max: range?.daemon_max ?? "?" })}
        </InlineAlert>
      ) : (
        <p className="first-run-body">{t(`first.${link}.body`)}</p>
      )}
      {command && (
        <div className="first-run-command-row">
          <code className="first-run-command">{command}</code>
          <Button icon={icons.copy} onClick={() => void navigator.clipboard.writeText(copyOf.current)}>
            {t("first.copy")}
          </Button>
        </div>
      )}
      {link === "permission_denied" && <InlineAlert tone="info" toneLabel={t("alert.info")}>{t("first.permission_denied.relogin")}</InlineAlert>}
      {link !== "connecting" && (
        <div className="first-run-actions">
          <Button variant="primary" onClick={onRetry}>
            {t("first.retry")}
          </Button>
        </div>
      )}
    </section>
  );
}
