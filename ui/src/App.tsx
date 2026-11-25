import { useEffect, useState } from "react";
import { AppMenu, type LangChoice } from "./app/AppMenu";
import { useDaemon } from "./daemon/store";
import { detectLang, I18n, useT } from "./i18n";
import type { Lang } from "./messages";
import { FirstRun } from "./screens/FirstRun";
import { StatusView } from "./screens/StatusView";

function Screen({ langChoice, onLangChoice }: { langChoice: LangChoice; onLangChoice: (choice: LangChoice) => void }) {
  const t = useT();
  const { view, info, retry } = useDaemon();
  const link = view?.link ?? "connecting";
  const barKey = `bar.${link}` as const;
  return (
    <>
      <header>
        <h1>{t("app.title")}</h1>
        <AppMenu langChoice={langChoice} onLangChoice={onLangChoice} info={info} daemonVersion={view?.daemon_version ?? null} />
      </header>
      <main>
        {view && info && link === "connected" ? (
          <StatusView view={view} />
        ) : view && info ? (
          <FirstRun view={view} info={info} onRetry={retry} />
        ) : (
          <p>{t("first.connecting.body")}</p>
        )}
      </main>
      <footer className={`bar bar-${link}`} aria-live="polite">
        {link === "connected" ? t("bar.connected", { version: view?.daemon_version ?? "?" }) : t(barKey)}
      </footer>
    </>
  );
}

/** T1: the chosen language lives here, in memory; T3 moves it into the preferences. */
export function App({ lang }: { lang?: Lang }) {
  const [langChoice, setLangChoice] = useState<LangChoice>(lang ?? "system");
  const language = langChoice === "system" ? detectLang() : langChoice;
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);
  return (
    <I18n lang={language}>
      <Screen langChoice={langChoice} onLangChoice={setLangChoice} />
    </I18n>
  );
}
