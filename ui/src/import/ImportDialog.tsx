import { useEffect, useRef, useState } from "react";
import { Button } from "../design/Button";
import { Dialog } from "../design/Dialog";
import { Field } from "../design/Field";
import { InlineAlert } from "../design/InlineAlert";
import { useToast } from "../design/Toast";
import { errorText, useT, type Translate } from "../i18n";
import type { CmdError, ProfileSummary } from "../rubrica/types";
import { issueText } from "../shell/tabs/tab";
import "./ImportDialog.css";
import { Preview } from "./Preview";
import type { ImportActions, ImportPreview, Incoming } from "./types";

type Focus = "text" | "choose" | "name" | "group" | "error";

/** The text of an error of the import: the codes that carry something the catalogue text lacks (the line, the file) say it. */
function problemText(t: Translate, error: CmdError): string {
  const data = (error.data ?? {}) as { line?: number; file?: string; origin?: string };
  if (error.code === "import_syntax" && typeof data.line === "number") return t("import.error.syntax_line", { line: data.line });
  if (error.code === "import_missing_file" && data.origin === "text") return t("import.error.missing_file_text", { file: data.file ?? "" });
  return errorText(t, error.code, error.message);
}

/** «Imported “X”.», followed by how many directives were dropped when there are some: one notice, said once (Q27). */
function outcomeText(t: Translate, name: string, removed: number): string {
  const done = t("import.done", { name });
  if (removed === 0) return done;
  return `${done} ${removed === 1 ? t("import.removed.one") : t("import.removed.other", { count: removed })}`;
}

/**
 * «Import profile» (spec §7): a modal window with two steps. First the source, here the text of the profile; then the preview
 * with what was found and what was dropped, the name and the group, and «Import». The source is a file, chosen in the
 * window «Open» that Rust opens (the page never has a path), or the text. Nothing is written before «Import»: the
 * preview waits in Rust, and «Cancel», `Esc`, the close button and «Back» let it go. An error of the import is an alert
 * inside the window and takes the focus (Q27: it is not announced, it is read where the focus is); a name Rust refuses puts
 * the focus on the field, with the reason as its description. The text is emptied as soon as the preview arrives.
 */
export function ImportDialog(props: {
  open: boolean;
  onClose: () => void;
  actions: ImportActions;
  /** The profile was saved and the rubrica read again: the shell selects it and puts the focus on its row. */
  onImported: (profile: ProfileSummary) => void;
  /** A file that came from outside (dropped, or given to the app): it opens the window with its preview or its error. */
  incoming?: Incoming | null;
}) {
  const { open, onClose, actions, onImported, incoming } = props;
  const t = useT();
  const toast = useToast();
  const body = useRef<HTMLDivElement>(null);
  const [text, setText] = useState("");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [error, setError] = useState<CmdError | null>(null);
  const [name, setName] = useState("");
  const [group, setGroup] = useState("");
  const [nameError, setNameError] = useState("");
  const [groupError, setGroupError] = useState("");
  const [busy, setBusy] = useState(false);
  // The window «Open» is up: the button that asked says so, and keeps the focus.
  const [choosing, setChoosing] = useState(false);
  // Where the focus goes once the step has been drawn; `turn` makes the same place count again.
  const [focus, setFocus] = useState<{ on: Focus; turn: number } | null>(null);
  const ask = (on: Focus) => setFocus((was) => ({ on, turn: (was?.turn ?? 0) + 1 }));

  // Every opening starts from the first step, with nothing left of the last one.
  useEffect(() => {
    if (!open) return;
    setText("");
    setPreview(null);
    setError(null);
    setName("");
    setGroup("");
    setNameError("");
    setGroupError("");
    setBusy(false);
    setChoosing(false);
    ask("text");
  }, [open]);

  useEffect(() => {
    if (!open || !focus) return undefined;
    // After the library has put the focus where it wants it (the first stop of the window)
    const timer = setTimeout(() => {
      const root = body.current;
      if (!root) return;
      const target: Record<Focus, string> = { text: "textarea", choose: ".import-choose button", name: 'input[name="import-name"]', group: 'select[name="import-group"]', error: ".import-error" };
      root.querySelector<HTMLElement>(target[focus.on])?.focus();
    }, 0);
    return () => clearTimeout(timer);
  }, [open, focus]);

  const letGo = (held: ImportPreview | null) => {
    if (held) void actions.discard(held.id);
  };
  const close = () => {
    letGo(preview);
    onClose();
  };
  const back = () => {
    letGo(preview);
    setPreview(null);
    setError(null);
    ask("text");
  };

  const fail = (failure: unknown, step: "source" | "preview") => {
    const problem = failure as CmdError;
    if (problem.code === "gui_import_expired") {
      // The preview is gone: the text is gone too, so the window goes back to the first step and says so.
      setPreview(null);
      setError(problem);
      ask("error");
      return;
    }
    if (step === "preview" && problem.code === "gui_invalid_field") {
      const field = problem.data?.fields?.find((f) => f.field === "name");
      if (field) {
        setNameError(issueText(t, "name", field.code));
        ask("name");
        return;
      }
    }
    if (step === "preview" && problem.code === "gui_group_not_found") {
      setGroupError(errorText(t, problem.code, problem.message));
      ask("group");
      return;
    }
    setError(problem);
    ask("error");
  };

  /** The second step, for a preview that came from the text or from the file. */
  const showPreview = (made: ImportPreview) => {
    // The text is the one thing of a profile that went through the page: it goes as soon as it has been read.
    setText("");
    setPreview(made);
    setName(made.suggested_name || t("import.default_name"));
    setGroup("");
    setNameError("");
    setGroupError("");
    ask("name");
  };

  // A file that came from outside (dropped on the window, or given to the app) takes the place of what the window shows
  // (P7): the preview that was there is let go, and the new one, or its error, is shown. The error is read where the focus is.
  useEffect(() => {
    if (!open || !incoming) return;
    letGo(preview);
    setText("");
    setBusy(false);
    setChoosing(false);
    if ("preview" in incoming.offer) {
      showPreview(incoming.offer.preview);
    } else {
      setPreview(null);
      setError(incoming.offer.failed.error);
      ask("error");
    }
    // only a new offer counts: the rest is read as it was when it arrived
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [incoming?.turn]);

  const makePreview = async () => {
    if (busy || choosing) return;
    setBusy(true);
    setError(null);
    try {
      showPreview(await actions.fromText(text));
    } catch (failure) {
      fail(failure, "source");
    } finally {
      setBusy(false);
    }
  };

  const chooseFile = async () => {
    if (busy || choosing) return;
    setChoosing(true);
    setError(null);
    try {
      const made = await actions.pickFile();
      // A cancelled window is not an outcome: nothing is said, and the focus goes back to the button that asked.
      if (made) showPreview(made);
      else ask("choose");
    } catch (failure) {
      fail(failure, "source");
    } finally {
      setChoosing(false);
    }
  };

  const importIt = async () => {
    if (busy || !preview) return;
    setBusy(true);
    setNameError("");
    setGroupError("");
    setError(null);
    try {
      const saved = await actions.commit(preview.id, name, group === "" ? null : group);
      toast(outcomeText(t, saved.name, preview.removed), { tone: "ok" });
      onClose();
      onImported(saved);
    } catch (failure) {
      fail(failure, "preview");
    } finally {
      setBusy(false);
    }
  };

  const step = preview ? "preview" : "source";
  const footer = preview ? (
    <>
      <Button onClick={close}>{t("import.cancel")}</Button>
      <Button onClick={back}>{t("import.back")}</Button>
      <Button variant="primary" busy={busy} onClick={() => void importIt()}>{t("import.confirm")}</Button>
    </>
  ) : (
    <>
      <Button onClick={close}>{t("import.cancel")}</Button>
      <Button variant="primary" busy={busy} disabled={text.trim() === "" || choosing} onClick={() => void makePreview()}>{t("import.preview.button")}</Button>
    </>
  );

  return (
    <Dialog
      open={open}
      onClose={close}
      title={t("import.title")}
      description={t(step === "preview" ? "import.description.preview" : "import.description.source")}
      closeLabel={t("dialog.close")}
      footer={footer}
    >
      <div className="import" ref={body}>
        {error ? (
          <div className="import-error" tabIndex={-1}>
            <InlineAlert tone="error" toneLabel={t("alert.error")} title={t("import.error.title")}>
              {problemText(t, error)}
            </InlineAlert>
          </div>
        ) : null}
        {preview ? (
          <form
            className="import-form"
            onSubmit={(event) => {
              event.preventDefault();
              void importIt();
            }}
          >
            <Preview preview={preview} />
            <Field name="import-name" label={t("import.name")} value={name} error={nameError || undefined} onChange={(value) => { setName(value); setNameError(""); }} />
            <Field
              as="select"
              name="import-group"
              label={t("import.group")}
              value={group}
              error={groupError || undefined}
              options={[{ value: "", label: t("group.none") }, ...preview.groups.map((g) => ({ value: g, label: g }))]}
              onChange={(value) => { setGroup(value); setGroupError(""); }}
            />
          </form>
        ) : (
          <>
            <div className="import-choose">
              <Button busy={choosing} disabled={busy} onClick={() => void chooseFile()}>{t("import.choose")}</Button>
              <p className="import-drop-hint">{t("import.drop_hint")}</p>
            </div>
            <Field as="textarea" mono rows={10} spellCheck={false} name="import-text" label={t("import.text.label")} value={text} onChange={(value) => { setText(value); }} />
          </>
        )}
      </div>
    </Dialog>
  );
}
