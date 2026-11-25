import { useEffect } from "react";
import { Button } from "../design/Button";
import { useT } from "../i18n";
import "./SaveBar.css";

/**
 * «Save» and «Cancel» for the text fields of a profile (P7, Q33): it is in the page only while there is something to
 * save, with a sentence that says so. `Ctrl+S` saves from anywhere in the window while it is there. It never announces
 * itself: what a screen reader needs to hear is the outcome of the save, which is the notice that follows.
 */
export function SaveBar({ dirty, busy, onSave, onCancel }: { dirty: boolean; busy: boolean; onSave: () => void; onCancel: () => void }) {
  const t = useT();
  useEffect(() => {
    if (!dirty) return undefined;
    const keydown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (!busy) onSave();
      }
    };
    document.addEventListener("keydown", keydown);
    return () => document.removeEventListener("keydown", keydown);
  }, [dirty, busy, onSave]);
  if (!dirty) return null;
  return (
    <div className="savebar" role="group" aria-label={t("savebar.label")}>
      <p className="savebar-text">{t("savebar.text")}</p>
      <Button variant="primary" busy={busy} onClick={onSave}>{t("savebar.save")}</Button>
      <Button onClick={onCancel} disabled={busy}>{t("savebar.cancel")}</Button>
    </div>
  );
}
