import { useId } from "react";
import "./Switch.css";

/** On/off. A native button with the `switch` role: Space and Enter toggle it, and a click on the label does too. */
export function Switch(props: { label: string; hint?: string; checked: boolean; disabled?: boolean; onChange: (checked: boolean) => void }) {
  const { label, hint, checked, disabled, onChange } = props;
  const id = useId();
  return (
    <label className="gx-switch">
      <button
        type="button"
        role="switch"
        className="gx-switch-track"
        aria-checked={checked}
        aria-labelledby={`${id}-label`}
        aria-describedby={hint ? `${id}-hint` : undefined}
        disabled={disabled}
        onClick={() => onChange(!checked)}
      >
        <span className="gx-switch-thumb" aria-hidden="true" />
      </button>
      <span className="gx-switch-text">
        <span id={`${id}-label`} className="gx-switch-label">{label}</span>
        {hint ? <span id={`${id}-hint`} className="gx-switch-hint">{hint}</span> : null}
      </span>
    </label>
  );
}
