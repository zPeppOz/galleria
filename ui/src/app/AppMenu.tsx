import { useState } from "react";
import type { AppInfo } from "../daemon/types";
import { Menu, type MenuEntry } from "../design/Menu";
import type { ThemeChoice } from "../design/theme";
import { useT } from "../i18n";
import { usePrefs, type LangChoice } from "../prefs";
import { AboutDialog } from "./AboutDialog";

/** The menu of the title bar: language, theme, and «About Galleria». Language and theme are kept by the preferences. */
export function AppMenu(props: { info: AppInfo | null; daemonVersion: string | null }) {
  const t = useT();
  const prefs = usePrefs();
  const [aboutOpen, setAboutOpen] = useState(false);
  const entries: MenuEntry[] = [
    {
      kind: "radio",
      id: "lang",
      label: t("menu.language"),
      value: prefs.lang,
      options: [
        { value: "system", label: t("menu.language.system") },
        { value: "en", label: t("menu.language.en") },
        { value: "it", label: t("menu.language.it") },
      ],
      onChange: (value) => prefs.setLang(value as LangChoice),
    },
    {
      kind: "radio",
      id: "theme",
      label: t("menu.theme"),
      value: prefs.theme,
      options: [
        { value: "system", label: t("menu.theme.system") },
        { value: "light", label: t("menu.theme.light") },
        { value: "dark", label: t("menu.theme.dark") },
      ],
      onChange: (value) => prefs.setTheme(value as ThemeChoice),
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
