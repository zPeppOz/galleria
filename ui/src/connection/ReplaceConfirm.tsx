import { ConfirmDialog } from "../design/ConfirmDialog";
import { useT } from "../i18n";
import { useConnection } from "./store";

/**
 * The question about a session that is not of this rubrica (D21): ending it, or ending it to connect a profile of the rubrica. It is an
 * `alertdialog`, so it takes the focus and announces nothing (Q27, rule 2), and the focus starts on «Cancel». Nothing is sent to the
 * service before the person says yes.
 */
export function ReplaceConfirm() {
  const t = useT();
  const { confirmation, answerConfirmation } = useConnection();
  const replacing = confirmation?.kind === "replace";
  const name = confirmation?.name ?? "";
  return (
    <ConfirmDialog
      open={confirmation !== null}
      danger
      title={t(replacing ? "foreign.title_replace" : "foreign.title", { name })}
      body={t(replacing ? "foreign.body_replace" : "foreign.body")}
      confirmLabel={t(replacing ? "foreign.replace" : "action.disconnect")}
      cancelLabel={t("action.cancel")}
      onConfirm={() => void answerConfirmation(true)}
      onCancel={() => void answerConfirmation(false)}
    />
  );
}
