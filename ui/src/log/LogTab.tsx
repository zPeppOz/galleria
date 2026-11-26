import { memo, useId, useLayoutEffect, useMemo, useRef } from "react";
import { isLive, useConnection } from "../connection/store";
import { logsExport, type CommandError } from "../daemon/commands";
import type { LogEntry } from "../daemon/types";
import { Button } from "../design/Button";
import { InlineAlert } from "../design/InlineAlert";
import { useToast } from "../design/Toast";
import { errorText, useLang, useT, type Lang } from "../i18n";
import { formatLine, useLog } from "./store";
import "./LogTab.css";

/** The time of a line in the language of the window, to the second. A time that is not a number of milliseconds is shown as it came. */
function clock(ts: string, lang: Lang): string {
  const millis = Number(ts);
  if (!Number.isFinite(millis)) return ts;
  return new Intl.DateTimeFormat(lang, { hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false }).format(new Date(millis));
}

const Line = memo(function Line({ entry, lang }: { entry: LogEntry; lang: Lang }) {
  return (
    <div className="log-line">
      <span className="log-time">{clock(entry.ts, lang)}</span>{" "}
      <span className={`log-level log-level-${entry.level}`}>{entry.level}</span>{" "}
      <span className="log-source">{entry.source}</span>{" "}
      <span className="log-message">{entry.message}</span>
    </div>
  );
});

/**
 * The tab «Log» of a profile: the lines of its session, the one that is up or the last one, as the service keeps them. The lines are shown
 * as they came (the filter is the service's, and a line is not translated); the level and the source are words. The list is **not** a
 * live region that announces (Q27, rule 3): it is a log with its announcements off, and it stays at the bottom only while the person
 * is at the bottom. «Copy» takes what the tab shows; «Export log…» takes nothing from the page: Rust reads the log again and asks the
 * system for the place (P11).
 */
export function LogTab({ profileId }: { profileId: string }) {
  const t = useT();
  const lang = useLang();
  const toast = useToast();
  const { status, lastSession } = useConnection();
  const titleId = useId();
  const live = isLive(status) && status.profile?.id === profileId;
  const last = !isLive(status) && lastSession?.id === profileId;
  const mine = live || last;
  const session = live ? (status?.session ?? null) : (lastSession?.session ?? null);
  const { lines, error } = useLog(mine, session);

  const box = useRef<HTMLDivElement>(null);
  const atBottom = useRef(true);
  const follow = () => {
    const el = box.current;
    if (el) atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 16;
  };
  useLayoutEffect(() => {
    const el = box.current;
    if (el && atBottom.current) el.scrollTop = el.scrollHeight;
  }, [lines]);

  const text = useMemo(() => lines.map(formatLine).join("\n"), [lines]);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text.length > 0 ? `${text}\n` : "");
      toast(t("log.copied"), { tone: "ok" });
    } catch {
      toast(t("log.copy_failed"), { tone: "error" });
    }
  };
  const exportLog = async () => {
    try {
      // a cancel of the window «Save» is not an outcome: nothing is said
      if ((await logsExport()).saved) toast(t("log.exported"), { tone: "ok" });
    } catch (failure) {
      const refused = failure as CommandError;
      toast(errorText(t, refused.code, refused.message), { tone: "error" });
    }
  };

  if (!mine) return <p className="log-empty">{t("log.none")}</p>;
  return (
    <section className="log-view" aria-labelledby={titleId}>
      <div className="log-head">
        <h2 id={titleId} className="log-title">{t(live ? "log.current" : "log.last")}</h2>
        <div className="log-actions">
          <Button onClick={() => void copy()}>{t("log.copy")}</Button>
          <Button onClick={() => void exportLog()}>{t("log.export")}</Button>
        </div>
      </div>
      {error ? (
        <InlineAlert tone="error" toneLabel={t("alert.error")}>
          {errorText(t, error.code, error.message)}
        </InlineAlert>
      ) : null}
      {/* a `log` is a polite live region by default: off, so that a screen reader says nothing for each line */}
      <div ref={box} className="log-lines" role="log" aria-live="off" aria-labelledby={titleId} tabIndex={0} onScroll={follow}>
        {lines.length === 0 ? <p className="log-empty">{t("log.empty")}</p> : lines.map((entry) => <Line key={entry.seq} entry={entry} lang={lang} />)}
      </div>
    </section>
  );
}
