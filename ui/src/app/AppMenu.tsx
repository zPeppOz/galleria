import { useState } from "react";
import type { AppInfo } from "../daemon/types";
import { Menu, type MenuEntry } from "../design/Menu";
import { useT } from "../i18n";
import type { Lang } from "../messages";
import { AboutDialog } from "./AboutDialog";

export type LangChoice = "system" | Lang;

/** The menu of the title bar: language, and «About Galleria». T3 adds the theme and reads the language from the preferences. */
export function AppMenu(props: { langChoice: LangChoice; onLangChoice: (choice: LangChoice) => void; info: AppInfo | null; daemonVersion: string | null }) {
  const t = useT();
  const [aboutOpen, setAboutOpen] = useState(false);
  const entries: MenuEntry[] = [
    {
      kind: "radio",
      id: "lang",
      label: t("menu.language"),
      value: props.langChoice,
      options: [
        { value: "system", label: t("menu.language.system") },
        { value: "en", label: t("menu.language.en") },
        { value: "it", label: t("menu.language.it") },
      ],
      onChange: (value) => props.onLangChoice(value as LangChoice),
    },
    { kind: "separator" },
    { kind: "item", id: "about", label: t("menu.about"), onSelect: () => setAboutOpen(true) },
  ];
  return (
    <>
      <Menu label={t("menu.label")} trigger={t("menu.label")} entries={entries} />
      <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} info={props.info} daemonVersion={props.daemonVersion} />
    </>
  );
}
