import { useRef, type FormEvent, type RefObject } from "react";
import type { InputAnswer } from "../daemon/commands";
import { Dialog } from "../design/Dialog";
import { useT } from "../i18n";
import "../design/Button.css";
import "../design/Field.css";
import "./CredentialsDialog.css";
import { formatClock } from "./format";
import { useConnection, useCountdown } from "./store";

/**
 * The window of the credentials: it opens when the service asks for something (`needs_input`) and shows only what was asked, by kind.
 *
 * What is typed does not live in this page. The fields are not controlled: the value is in the `<input>` and nowhere else, there is no
 * state, no store and no closure that outlives the click. On submit the values are read once, the fields are emptied, and only then
 * is the answer given to the connection (which makes the one call to Rust and keeps nothing). Nothing is logged. The file has no
 * component state, and a test reads it to see that stays true (`the_credentials_dialog_keeps_no_value_in_state`).
 *
 * It takes the focus and says nothing by itself (Q27, rule 2); the time left is a text that is not announced. A click on the veil
 * does not close it, «Cancel» is a disconnect (P7), and at zero it closes and the state that arrives says the rest.
 */
export function CredentialsDialog() {
  const t = useT();
  const { question, provide, disconnect } = useConnection();
  const left = useCountdown(question?.expiresAt ?? null);
  const username = useRef<HTMLInputElement>(null);
  const password = useRef<HTMLInputElement>(null);
  const code = useRef<HTMLInputElement>(null);
  const passphrase = useRef<HTMLInputElement>(null);
  const remember = useRef<HTMLInputElement>(null);
  if (!question || left <= 0) return null;

  const first: RefObject<HTMLInputElement | null> = question.kind === "credentials" ? username : question.kind === "otp" ? code : passphrase;
  const take = (field: RefObject<HTMLInputElement | null>): string => {
    const value = field.current?.value ?? "";
    if (field.current) field.current.value = "";
    return value;
  };
  const submit = (event: FormEvent) => {
    event.preventDefault();
    const values: InputAnswer = {};
    if (question.kind === "credentials") {
      values.username = take(username);
      values.password = take(password);
    } else if (question.kind === "otp") {
      values.otp = take(code);
    } else {
      values.key_passphrase = take(passphrase);
    }
    const keep = question.kind === "credentials" && remember.current?.checked === true;
    if (remember.current) remember.current.checked = false;
    void provide(values, keep);
  };
  const type = question.echo ? "text" : "password";
  const field = (ref: RefObject<HTMLInputElement | null>, label: string, kind: "text" | "password", name: string) => (
    <div className="gx-field">
      <label className="gx-field-label" htmlFor={`credentials-${name}`}>{label}</label>
      <input
        id={`credentials-${name}`}
        name={name}
        ref={ref}
        className="gx-control"
        type={kind}
        required
        autoComplete="off"
        spellCheck={false}
        autoCapitalize="off"
      />
    </div>
  );
  return (
    <Dialog open onClose={() => void disconnect()} title={t(`credentials.title.${question.kind}`, { name: question.profile.name })} closeLabel={t("dialog.close")} ignoreOutsidePress initialFocus={first}>
      <form className="credentials-form" onSubmit={submit} data-testid="credentials-form">
        {question.kind === "credentials" ? (
          <>
            {field(username, t("credentials.username"), "text", "username")}
            {field(password, t("credentials.password"), "password", "password")}
            <div className="gx-field">
              <label className="gx-field-label">
                <input ref={remember} type="checkbox" name="remember" /> {t("credentials.remember")}
              </label>
              <p className="gx-field-hint">{t("credentials.remember.hint")}</p>
            </div>
          </>
        ) : null}
        {question.kind === "otp" ? field(code, question.prompt, type, "otp") : null}
        {question.kind === "key_passphrase" ? field(passphrase, t("credentials.passphrase"), type, "passphrase") : null}
        <p className="gx-field-hint credentials-countdown">{t("credentials.countdown", { time: formatClock(left) })}</p>
        <div className="gx-dialog-footer">
          <button type="button" className="gx-button gx-button-default gx-button-md" onClick={() => void disconnect()}>{t("action.cancel")}</button>
          <button type="submit" className="gx-button gx-button-primary gx-button-md">{t("action.connect")}</button>
        </div>
      </form>
    </Dialog>
  );
}
