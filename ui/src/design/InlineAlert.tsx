import type { ReactNode } from "react";
import { icons } from "./icons";
import "./InlineAlert.css";

const TONE_ICON = { info: icons.info, ok: icons.check, warn: icons.warning, error: icons.error } as const;

/**
 * A notice inside the page. The tone shows in the icon and the border, and the icon has a name (`toneLabel`), so the
 * state never rests on colour alone. Only an error is an `alert`; the other tones are `status` (polite).
 */
export function InlineAlert(props: {
  tone: "info" | "ok" | "warn" | "error";
  /** The word for the tone («Error», «Warning»…): the accessible name of the icon. */
  toneLabel: string;
  title?: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  const { tone, toneLabel, title, children, actions } = props;
  return (
    <div className={`gx-alert gx-alert-${tone}`} role={tone === "error" ? "alert" : "status"}>
      <span className="gx-alert-icon" role="img" aria-label={toneLabel}>{TONE_ICON[tone]}</span>
      <div className="gx-alert-body">
        {title ? <p className="gx-alert-title">{title}</p> : null}
        <div className="gx-alert-text">{children}</div>
        {actions ? <div className="gx-alert-actions">{actions}</div> : null}
      </div>
    </div>
  );
}
