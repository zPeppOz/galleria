import { Menu as BaseMenu } from "@base-ui/react/menu";
import { useId, type ReactNode } from "react";
import { icons } from "./icons";
import "./overlays.css";

export type MenuEntry =
  /** `disabled` keeps the item where the arrows can reach it, so that its `hint` (why it is off) is read: it is not run and the menu stays open. */
  | { kind: "item"; id: string; label: string; onSelect: () => void; danger?: boolean; disabled?: boolean; hint?: string }
  | { kind: "radio"; id: string; label: string; value: string; options: { value: string; label: string }[]; onChange: (v: string) => void }
  | { kind: "separator" };

/** Menu opened by a button. `label` is the accessible name of the menu itself; `trigger` is what the button shows. */
export function Menu(props: { label: string; trigger: ReactNode; entries: MenuEntry[] }) {
  const { label, trigger, entries } = props;
  const menuId = useId();
  return (
    <BaseMenu.Root>
      <BaseMenu.Trigger className="gx-menu-trigger">{trigger}</BaseMenu.Trigger>
      <BaseMenu.Portal>
        <BaseMenu.Positioner className="gx-menu-positioner" sideOffset={6} align="end">
          <BaseMenu.Popup className="gx-menu" aria-label={label}>
            {entries.map((entry, index) => {
              if (entry.kind === "separator") return <BaseMenu.Separator key={`sep-${index}`} className="gx-menu-separator" />;
              if (entry.kind === "item") {
                const hintId = entry.hint ? `${menuId}-${entry.id}-hint` : undefined;
                return (
                  <BaseMenu.Item
                    key={entry.id}
                    className={entry.danger ? "gx-menu-item gx-danger" : "gx-menu-item"}
                    // A switched-off item is still a stop of the arrows (it says why), and neither runs nor closes the menu.
                    closeOnClick={!entry.disabled}
                    aria-disabled={entry.disabled ? true : undefined}
                    // The name is the label alone; the reason is the description.
                    aria-label={hintId ? entry.label : undefined}
                    aria-describedby={hintId}
                    onClick={() => {
                      if (!entry.disabled) entry.onSelect();
                    }}
                  >
                    {entry.label}
                    {entry.hint ? <span id={hintId} className="gx-menu-hint">{entry.hint}</span> : null}
                  </BaseMenu.Item>
                );
              }
              return (
                <BaseMenu.Group key={entry.id} className="gx-menu-group">
                  <BaseMenu.GroupLabel className="gx-menu-group-label">{entry.label}</BaseMenu.GroupLabel>
                  <BaseMenu.RadioGroup value={entry.value} onValueChange={(value) => entry.onChange(String(value))}>
                    {entry.options.map((option) => (
                      <BaseMenu.RadioItem key={option.value} value={option.value} closeOnClick className="gx-menu-item gx-menu-radio">
                        <span className="gx-menu-indicator">
                          <BaseMenu.RadioItemIndicator>{icons.check}</BaseMenu.RadioItemIndicator>
                        </span>
                        {option.label}
                      </BaseMenu.RadioItem>
                    ))}
                  </BaseMenu.RadioGroup>
                </BaseMenu.Group>
              );
            })}
          </BaseMenu.Popup>
        </BaseMenu.Positioner>
      </BaseMenu.Portal>
    </BaseMenu.Root>
  );
}
