import { useId } from "react";
import "./Segmented.css";

/**
 * A group of choices of which one is on: native radio buttons, so the arrows move the choice and the group is one
 * stop for the Tab key. `readOnly` shows the value and ignores the change.
 */
export function Segmented(props: {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  readOnly?: boolean;
}) {
  const { label, value, options, onChange, readOnly } = props;
  const name = useId();
  return (
    <div className="gx-segmented" role="radiogroup" aria-label={label} aria-readonly={readOnly ? true : undefined}>
      {options.map((option) => (
        <label key={option.value} className="gx-segmented-option">
          <input
            type="radio"
            name={name}
            value={option.value}
            checked={option.value === value}
            onChange={() => {
              if (!readOnly) onChange(option.value);
            }}
          />
          <span className="gx-segmented-label">{option.label}</span>
        </label>
      ))}
    </div>
  );
}
