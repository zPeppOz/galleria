import { useId } from "react";
import "./Field.css";

export interface FieldProps {
  label: string;
  hint?: string;
  error?: string;
  mono?: boolean;
  readOnly?: boolean;
  value: string;
  onChange?: (value: string) => void;
  as?: "input" | "textarea" | "select";
  options?: { value: string; label: string }[];
  type?: "text" | "password";
  inputMode?: "numeric";
  /** The `name` of the control: how a form finds the field to put the focus on. */
  name?: string;
}

/** Label, hint and error are linked to the control (`for`, `aria-describedby`, `aria-invalid`). */
export function Field(props: FieldProps) {
  const { label, hint, error, mono, readOnly, value, onChange, as = "input", options = [], type = "text", inputMode, name } = props;
  const id = useId();
  const hintId = hint ? `${id}-hint` : undefined;
  const errorId = error ? `${id}-error` : undefined;
  const describedBy = [hintId, errorId].filter(Boolean).join(" ") || undefined;
  const common = {
    id,
    name,
    className: `gx-control${mono ? " gx-mono" : ""}`,
    "aria-describedby": describedBy,
    "aria-invalid": error ? true : undefined,
  };
  const change = (event: { target: { value: string } }) => onChange?.(event.target.value);
  return (
    <div className="gx-field">
      <label className="gx-field-label" htmlFor={id}>{label}</label>
      {as === "select" ? (
        // A read-only list shows the chosen option only: a native select has no read-only state.
        <select {...common} value={value} onChange={change} aria-readonly={readOnly ? true : undefined}>
          {options.filter((o) => !readOnly || o.value === value).map((o) => (
            <option key={o.value} value={o.value}>{o.label}</option>
          ))}
        </select>
      ) : as === "textarea" ? (
        <textarea {...common} value={value} onChange={change} readOnly={readOnly} rows={4} />
      ) : (
        <input
          {...common}
          type={type}
          value={value}
          onChange={change}
          readOnly={readOnly}
          inputMode={inputMode}
          autoComplete={type === "password" ? "off" : undefined}
          spellCheck={type === "password" ? false : undefined}
        />
      )}
      {hint ? <p id={hintId} className="gx-field-hint">{hint}</p> : null}
      {error ? <p id={errorId} className="gx-field-error">{error}</p> : null}
    </div>
  );
}
