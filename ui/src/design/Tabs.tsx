import { Tabs as BaseTabs } from "@base-ui/react/tabs";
import type { ReactNode } from "react";
import "./Tabs.css";

/** Tabs: the arrows, Home and End move between them and select; the Tab key goes into the panel. Only the selected panel is drawn. */
export function Tabs(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  tabs: { id: string; label: string; panel: ReactNode }[];
}) {
  const { label, value, onChange, tabs } = props;
  return (
    <BaseTabs.Root className="gx-tabs" value={value} onValueChange={(next) => onChange(String(next))}>
      <BaseTabs.List className="gx-tablist" aria-label={label} activateOnFocus>
        {tabs.map((tab) => (
          <BaseTabs.Tab key={tab.id} value={tab.id} className="gx-tab">{tab.label}</BaseTabs.Tab>
        ))}
        <BaseTabs.Indicator className="gx-tab-indicator" />
      </BaseTabs.List>
      {tabs.map((tab) => (
        <BaseTabs.Panel key={tab.id} value={tab.id} className="gx-tabpanel">{tab.panel}</BaseTabs.Panel>
      ))}
    </BaseTabs.Root>
  );
}
