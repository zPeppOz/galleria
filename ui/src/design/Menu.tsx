import { Menu as BaseMenu } from "@base-ui/react/menu";
import type { ReactNode } from "react";
import { icons } from "./icons";
import "./overlays.css";

export type MenuEntry =
  | { kind: "item"; id: string; label: string; onSelect: () => void; danger?: boolean }
  | { kind: "radio"; id: string; label: string; value: string; options: { value: string; label: string }[]; onChange: (v: string) => void }
  | { kind: "separator" };

/** Menu opened by a button. `label` is the accessible name of the menu itself; `trigger` is what the button shows. */
export function Menu(props: { label: string; trigger: ReactNode; entries: MenuEntry[] }) {
  const { label, trigger, entries } = props;
  return (
    <BaseMenu.Root>
      <BaseMenu.Trigger className="gx-menu-trigger">{trigger}</BaseMenu.Trigger>
      <BaseMenu.Portal>
        <BaseMenu.Positioner className="gx-menu-positioner" sideOffset={6} align="end">
          <BaseMenu.Popup className="gx-menu" aria-label={label}>
            {entries.map((entry, index) => {
              if (entry.kind === "separator") return <BaseMenu.Separator key={`sep-${index}`} className="gx-menu-separator" />;
              if (entry.kind === "item") {
                return (
                  <BaseMenu.Item key={entry.id} className={entry.danger ? "gx-menu-item gx-danger" : "gx-menu-item"} onClick={entry.onSelect}>
                    {entry.label}
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
