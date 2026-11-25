import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import type { ReactNode } from "react";
import { icons } from "./icons";
import "./overlays.css";

/**
 * Modal dialog. Our own props: no type of the headless library leaves this file (the boundary test checks that).
 * Title and description are linked with aria-labelledby / aria-describedby by the library; focus enters the dialog
 * on open, stays inside while it is open and returns to the element that had it on close.
 */
export function Dialog(props: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  closeLabel: string;
  footer?: ReactNode;
  children: ReactNode;
}) {
  const { open, onClose, title, description, closeLabel, footer, children } = props;
  return (
    <BaseDialog.Root open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <BaseDialog.Portal>
        <BaseDialog.Backdrop className="gx-backdrop" />
        <BaseDialog.Popup className="gx-dialog">
          <div className="gx-dialog-head">
            <BaseDialog.Title className="gx-dialog-title">{title}</BaseDialog.Title>
            <BaseDialog.Close className="gx-icon-button" aria-label={closeLabel}>
              {icons.close}
            </BaseDialog.Close>
          </div>
          {description ? <BaseDialog.Description className="gx-dialog-description">{description}</BaseDialog.Description> : null}
          <div className="gx-dialog-body">{children}</div>
          {footer ? <div className="gx-dialog-footer">{footer}</div> : null}
        </BaseDialog.Popup>
      </BaseDialog.Portal>
    </BaseDialog.Root>
  );
}
