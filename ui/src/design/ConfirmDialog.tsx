import { AlertDialog } from "@base-ui/react/alert-dialog";
import { useRef } from "react";
import "./Button.css";
import "./overlays.css";

/**
 * Asks before something that cannot be undone. An `alertdialog`: named by the title, described by the body. The focus
 * starts on the safe choice (Cancel) when the action is dangerous, and on the action otherwise; Escape cancels.
 * The two buttons are plain `<button>`s with the classes of `Button`, because the focus is placed on them by reference.
 */
export function ConfirmDialog(props: {
  open: boolean;
  title: string;
  body: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  /** A third choice between the two (Cancel, «Discard», Confirm): a button of the default look. */
  alternateLabel?: string;
  onAlternate?: () => void;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { open, title, body, confirmLabel, cancelLabel, danger, alternateLabel, onAlternate, onConfirm, onCancel } = props;
  const cancel = useRef<HTMLButtonElement | null>(null);
  const confirm = useRef<HTMLButtonElement | null>(null);
  return (
    <AlertDialog.Root open={open} onOpenChange={(next) => { if (!next) onCancel(); }}>
      <AlertDialog.Portal>
        <AlertDialog.Backdrop className="gx-backdrop" />
        <AlertDialog.Popup className="gx-dialog" initialFocus={danger ? cancel : confirm}>
          <AlertDialog.Title className="gx-dialog-title">{title}</AlertDialog.Title>
          <AlertDialog.Description className="gx-dialog-description">{body}</AlertDialog.Description>
          <div className="gx-dialog-footer">
            <button ref={cancel} type="button" className="gx-button gx-button-default gx-button-md" onClick={onCancel}>{cancelLabel}</button>
            {alternateLabel ? <button type="button" className="gx-button gx-button-default gx-button-md" onClick={onAlternate}>{alternateLabel}</button> : null}
            <button ref={confirm} type="button" className={`gx-button gx-button-${danger ? "danger" : "primary"} gx-button-md`} onClick={onConfirm}>{confirmLabel}</button>
          </div>
        </AlertDialog.Popup>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
