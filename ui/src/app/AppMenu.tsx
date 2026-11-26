import { useEffect, useState } from "react";
import { useConnection, isLive } from "../connection/store";
import type { AppInfo } from "../daemon/types";
import { Menu, type MenuEntry } from "../design/Menu";
import { useToast } from "../design/Toast";
import type { ThemeChoice } from "../design/theme";
import { errorText, useT } from "../i18n";
import { usePrefs, type LangChoice } from "../prefs";
import { AboutDialog } from "./AboutDialog";
import { quitApp } from "./closing";

/** The menu of the title bar: language, theme, «Quit» and «About Galleria». Language and theme are kept by the preferences. `Ctrl+Q` quits as «Quit» does. */
export function AppMenu(props: { info: AppInfo | null; daemonVersion: string | null }) {
  const t = useT();
  const prefs = usePrefs();
  const toast = useToast();
  const connection = useConnection();
  const [aboutOpen, setAboutOpen] = useState(false);
  // `Ctrl+Q` is the same request as the entry of the menu: Rust decides (changes that are not saved stop it and the window asks, Q35).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey && event.key.toLowerCase() === "q") {
        event.preventDefault();
        quitApp();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);
  // The choice applies at once; if it cannot be written the user is told it lasts only until the window closes.
  const saved = (write: Promise<void>) => {
    write.catch((error: unknown) => {
      const { code, message } = (error ?? {}) as { code?: string; message?: string };
      toast(errorText(t, code ?? "gui_state_not_saved", message ?? ""), { tone: "error" });
    });
  };
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
      onChange: (value) => saved(prefs.setLang(value as LangChoice)),
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
      onChange: (value) => saved(prefs.setTheme(value as ThemeChoice)),
    },
    { kind: "separator" },
    // The session belongs to the service: quitting leaves it up, and the entry says so while there is one (Q54).
    { kind: "item", id: "quit", label: isLive(connection.status) ? t("tray.quit_connected") : t("tray.quit"), onSelect: quitApp },
    { kind: "item", id: "about", label: t("menu.about"), onSelect: () => setAboutOpen(true) },
  ];
  return (
    <>
      <Menu label={t("menu.label")} trigger={t("menu.label")} entries={entries} />
      <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} info={props.info} daemonVersion={props.daemonVersion} />
    </>
  );
}
