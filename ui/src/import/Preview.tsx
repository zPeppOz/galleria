import { useT, type Translate } from "../i18n";
import { runsAScript } from "../rubrica/scripts";
import { ProtocolTag } from "../shell/ProtocolTag";
import { secretLabel } from "../shell/tabs/tab";
import "./Preview.css";
import type { ImportNotice, ImportPreview } from "./types";

/**
 * The reason a directive was dropped, in the words of D19 (spec §7): a directive that runs a script says that Galleria does
 * not run the scripts of profiles, and then, for the two scripts that set the DNS of the server, that Galleria applies the
 * DNS itself; the other dangerous directives and the unsupported ones have a sentence of their own.
 */
export function noticeReason(t: Translate, notice: ImportNotice): string {
  if (notice.kind === "unsupported") return t("import.notice.unsupported");
  if (notice.kind === "no_tls_client") return t("import.notice.no_client");
  if (!runsAScript(notice.directive)) return t("import.notice.dangerous");
  return t(notice.dns_script ? "import.notice.script_dns" : "import.notice.script");
}

/** One line: «Line 7 · Directive `up` **removed**: …». The name of the directive is all that is shown of it, never what follows. */
function Notice({ notice }: { notice: ImportNotice }) {
  const t = useT();
  if (notice.kind === "no_tls_client") return <li className="import-notice">{noticeReason(t, notice)}</li>;
  return (
    <li className="import-notice">
      {t("import.notice.where", { line: notice.line })} <code>{notice.directive}</code> <strong>{t("import.notice.removed")}</strong> {noticeReason(t, notice)}
    </li>
  );
}

/** How many peers the secrets say there are, for «shared key of peer 2»: the highest index, plus one. */
function peersOf(found: string[]): number {
  const indexes = found.flatMap((ref) => {
    const match = /^wireguard\.peer\.(\d+)\./.exec(ref);
    return match ? [Number(match[1])] : [];
  });
  return indexes.length === 0 ? 1 : Math.max(...indexes) + 1;
}

/**
 * What an import found, read only: the file and the protocol, the **names** of the secrets (never their values: the
 * preview has none) and where they will be kept, whether the credentials will be asked, and what the import dropped or
 * noticed, each with its line and the name of the directive. The name and the group are the fields below it.
 */
export function Preview({ preview }: { preview: ImportPreview }) {
  const t = useT();
  const peers = peersOf(preview.found);
  const names = preview.found.map((ref) => secretLabel(t, ref, peers)).join(", ");
  return (
    <section className="import-preview" aria-label={t("import.title")}>
      <p className="import-summary">
        <ProtocolTag protocol={preview.protocol} />
        {preview.file_name ? <span className="import-file">{t("import.preview.file", { name: preview.file_name })}</span> : null}
        {preview.server ? <span className="import-server">{preview.server}</span> : null}
      </p>
      <p className="import-found">{preview.found.length > 0 ? t("import.found", { names }) : t("import.found.none")}</p>
      {preview.auth.user_pass ? <p className="import-found">{t("import.auth.user_pass")}</p> : null}
      {preview.notices.length > 0 ? (
        <>
          <h3 className="import-subtitle">{t("import.notices")}</h3>
          <ul className="import-notices">
            {preview.notices.map((notice) => (
              <Notice key={`${notice.line}:${notice.directive}:${notice.kind}`} notice={notice} />
            ))}
          </ul>
        </>
      ) : null}
    </section>
  );
}
