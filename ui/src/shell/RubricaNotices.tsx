import { InlineAlert } from "../design/InlineAlert";
import { Button } from "../design/Button";
import { errorText, hasKey, useT } from "../i18n";
import type { Rubrica } from "../rubrica/store";
import type { SecretStoreKind } from "../secrets/store";
import { CommandRow } from "./CommandRow";
import "./RubricaNotices.css";

/**
 * What the rubrica has to say apart from the profiles: the store that cannot be read (with the command to fix it, when
 * there is one), the files that were skipped, and the empty rubrica with the button to import a profile. When the session
 * has no keyring the keys go to a file: a notice that stays says so (it cannot be dismissed, and the app reads it again when
 * the window comes back to the front).
 */
export function RubricaNotices({ rubrica, onImport, secretStore = null }: { rubrica: Rubrica; onImport: () => void; secretStore?: SecretStoreKind | null }) {
  const t = useT();
  const { error, view } = rubrica;
  const skipped = view?.skipped ?? [];
  return (
    <>
      {secretStore === "file" ? (
        <div className="rubrica-notice" data-notice="secret-store-file">
          <InlineAlert tone="warn" toneLabel={t("alert.warn")}>
            {t("secrets.file_notice")}
          </InlineAlert>
        </div>
      ) : null}
      {error ? (
        <div className="rubrica-notice">
          <InlineAlert tone="error" toneLabel={t("alert.error")} title={t("rubrica.error.title")}>
            {errorText(t, error.code, error.message)}
            {error.data?.path ? <span className="rubrica-notice-path">{t("rubrica.error.where", { path: error.data.path })}</span> : null}
          </InlineAlert>
          {error.data?.fix ? (
            <>
              <p className="rubrica-notice-text">{t("rubrica.error.fix")}</p>
              <CommandRow command={error.data.fix} />
            </>
          ) : null}
        </div>
      ) : null}
      {skipped.length > 0 ? (
        <div className="rubrica-notice">
          <InlineAlert tone="warn" toneLabel={t("alert.warn")} title={t("rubrica.skipped.title")}>
            <ul className="rubrica-skipped">
              {skipped.map((file) => (
                <li key={file.file_name}>
                  <code>{file.file_name}</code> — {hasKey(`error.${file.code}`) ? errorText(t, file.code, file.code) : file.code}
                </li>
              ))}
            </ul>
          </InlineAlert>
        </div>
      ) : null}
      {rubrica.loaded && view && view.profiles.length === 0 && !error ? (
        <section className="rubrica-notice" aria-labelledby="rubrica-empty-title">
          <h1 id="rubrica-empty-title" className="rubrica-empty-title">{t("rubrica.empty.title")}</h1>
          <p className="rubrica-notice-text">{t("rubrica.empty.body")}</p>
          <div><Button variant="primary" onClick={onImport}>{t("import.empty")}</Button></div>
        </section>
      ) : null}
    </>
  );
}
