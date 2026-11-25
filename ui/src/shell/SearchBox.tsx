import { useEffect, useId, useRef, type KeyboardEvent } from "react";
import { useT } from "../i18n";
import "./SearchBox.css";

/**
 * The search field above the list. The label is for the screen reader only; `Ctrl+K` brings the focus here from anywhere,
 * `Esc` empties the field, `↓` goes on to the list. The count of the results below is visible text in a `status`
 * element, and nothing announces it: Q27 keeps the live regions silent and does not send this one through `announce` (the
 * limit is declared in the spec, §12.1 rule 3).
 */
export function SearchBox(props: { value: string; onChange: (value: string) => void; count: number; onArrowDown: () => void }) {
  const { value, onChange, count, onArrowDown } = props;
  const t = useT();
  const id = useId();
  const field = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey && event.key.toLowerCase() === "k") {
        event.preventDefault();
        field.current?.focus();
        field.current?.select();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape" && value !== "") {
      event.preventDefault();
      onChange("");
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      onArrowDown();
    }
  };

  const searching = value.trim() !== "";
  return (
    <div className="search-box">
      <label className="visually-hidden" htmlFor={id}>{t("search.label")}</label>
      <input
        ref={field}
        id={id}
        className="gx-control search-box-field"
        type="search"
        value={value}
        placeholder={t("search.placeholder")}
        aria-keyshortcuts="Control+K"
        autoComplete="off"
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
      />
      <p className="search-count" role="status">{searching ? (count === 1 ? t("search.count.one") : t("search.count.other", { count })) : ""}</p>
    </div>
  );
}
