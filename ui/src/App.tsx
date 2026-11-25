import { useEffect } from "react";
import { useDaemon } from "./daemon/store";
import { detectLang, I18n, useT } from "./i18n";
import type { Lang } from "./messages";
import { FirstRun } from "./screens/FirstRun";
import { StatusView } from "./screens/StatusView";

function Screen() {
  const t = useT();
  const { view, info, retry } = useDaemon();
  const link = view?.link ?? "connecting";
  const barKey = `bar.${link}` as const;
  return (
    <>
      <header>
        <h1>{t("app.title")}</h1>
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

export function App({ lang }: { lang?: Lang }) {
  const language = lang ?? detectLang();
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);
  return (
    <I18n lang={language}>
      <Screen />
    </I18n>
  );
}
