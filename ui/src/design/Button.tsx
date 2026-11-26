import type { ReactNode } from "react";
import "./Button.css";

type Common = {
  variant?: "default" | "primary" | "danger" | "ghost";
  size?: "md" | "lg";
  icon?: ReactNode;
  disabled?: boolean;
  /** Working: the button keeps the focus and says so (`aria-busy`), and a click does nothing until it is over. */
  busy?: boolean;
  onClick?: () => void;
  /** The id of the text that says why the button is off, or what it does beyond its name. */
  "aria-describedby"?: string;
};
/** A button with no text needs an accessible name: the type forbids one without `aria-label`. */
export type ButtonProps = Common & ({ children: ReactNode; "aria-label"?: string } | { children?: undefined; "aria-label": string });

export function Button(props: ButtonProps) {
  const { variant = "default", size = "md", icon, disabled, busy, onClick, children } = props;
  return (
    <button
      type="button"
      className={`gx-button gx-button-${variant} gx-button-${size}${children === undefined ? " gx-button-icon-only" : ""}`}
      disabled={disabled}
      aria-busy={busy ? true : undefined}
      aria-label={props["aria-label"]}
      aria-describedby={props["aria-describedby"]}
      onClick={() => {
        if (!disabled && !busy) onClick?.();
      }}
    >
      {busy ? <span className="gx-spinner" aria-hidden="true" /> : icon}
      {children}
    </button>
  );
}
