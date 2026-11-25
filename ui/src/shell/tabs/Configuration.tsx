import { useRef } from "react";
import { Field } from "../../design/Field";
import { useT, type MessageKey, type Translate } from "../../i18n";
import type { RawIssue, RawIssueKind } from "../../rubrica/types";
import "./Configuration.css";
import { secretLabel, type TabProps } from "./tab";

/**
 * The directives that run a script of the profile (D19): they get the sentence of D19. The other directives that the
 * check calls dangerous (`log`, `user`, `auth-user-pass file`…) get a text of their own, because they do not run one.
 */
const SCRIPT_DIRECTIVES = new Set(["up", "down", "route-up", "route-pre-down", "ipchange", "tls-verify", "client-connect", "client-disconnect", "learn-address", "auth-user-pass-verify", "plugin", "up-restart", "dns-updown"]);

const KIND_TEXT: Record<RawIssueKind, MessageKey> = {
  syntax: "config.issue.syntax",
  dangerous: "config.issue.dangerous",
  unsupported: "config.issue.unsupported",
  daemon_owned: "config.issue.daemon_owned",
  file_reference: "config.issue.file_reference",
  secret_block: "config.issue.secret_block",
  no_tls_client: "config.issue.no_tls_client",
};

/** «Line 2 · up · Galleria does not run the scripts of profiles.» The directive is a name, never what was typed after it. */
export function issueLine(t: Translate, issue: RawIssue): string {
  const text = issue.kind === "dangerous" && SCRIPT_DIRECTIVES.has(issue.directive) ? t("config.issue.script") : t(KIND_TEXT[issue.kind]);
  if (issue.line === 0) return text;
  return issue.directive ? t("config.issue.line", { line: issue.line, directive: issue.directive, text }) : t("config.issue.line_only", { line: issue.line, text });
}

/** What the tab is given on top of the form: the findings of the last check of the text in the editor. */
export interface ConfigurationProps extends TabProps {
  issues: RawIssue[];
}

/**
 * «Configuration» (T7, Q32). An OpenVPN profile has its text in an editor, saved as it is written; below it the list of
 * what stops a save, each line a button that puts the caret on the line it talks about. A WireGuard profile shows its
 * `.conf` without the key lines, read only: every setting of it has a field in the other tabs. Both say, under the text,
 * which secrets the profile keeps in the archive.
 */
export function Configuration({ detail, draft, edit, errors, issues }: ConfigurationProps) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);
  const peers = detail.wireguard?.peers.length ?? 0;
  const names = detail.secret_refs.map((ref) => secretLabel(t, ref, peers)).join(", ");

  // The caret goes to the line and the line is selected, so that it can be seen and typed over.
  const goTo = (line: number) => {
    const area = box.current?.querySelector<HTMLTextAreaElement>("textarea");
    if (!area) return;
    const lines = area.value.split("\n");
    const start = lines.slice(0, line - 1).reduce((n, text) => n + text.length + 1, 0);
    area.focus();
    area.setSelectionRange(start, start + (lines[line - 1]?.length ?? 0));
    const height = Number.parseFloat(getComputedStyle(area).lineHeight);
    if (Number.isFinite(height)) area.scrollTop = Math.max(0, (line - 1) * height - area.clientHeight / 2);
  };

  return (
    <div className="detail-form config-form" ref={box}>
      {draft.config !== null ? (
        <Field
          name="openvpn_config"
          label={t("field.configuration")}
          hint={t("config.hint.openvpn")}
          error={errors.openvpn_config}
          as="textarea"
          mono
          rows={16}
          spellCheck={false}
          value={draft.config}
          onChange={(config) => edit((d) => ({ ...d, config }), "openvpn_config")}
        />
      ) : (
        <Field name="wireguard_config" label={t("field.configuration")} hint={t("config.hint.wireguard")} as="textarea" mono readOnly rows={14} spellCheck={false} value={detail.wireguard_config ?? ""} />
      )}
      {issues.length > 0 ? (
        <ul className="config-issues" aria-label={t("config.issues")}>
          {issues.map((issue, i) => (
            // eslint-disable-next-line react/no-array-index-key
            <li key={i}>
              {issue.line > 0 ? (
                // The first one is where the focus goes when a save is refused.
                <button type="button" name={i === 0 ? "config_issue" : undefined} className={`config-issue${issue.kind === "no_tls_client" ? " config-issue-warning" : ""}`} onClick={() => goTo(issue.line)}>
                  {issueLine(t, issue)}
                </button>
              ) : (
                <p className="config-issue-text">{issueLine(t, issue)}</p>
              )}
            </li>
          ))}
        </ul>
      ) : null}
      <p className="config-secrets">{names ? t("config.secrets", { names }) : t("config.secrets.none")}</p>
    </div>
  );
}
