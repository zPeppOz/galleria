import { AppMenu } from "./app/AppMenu";
import { useDaemon } from "./daemon/store";
import { ToastProvider } from "./design/Toast";
import { useT } from "./i18n";
import { Preferences, type LangChoice } from "./prefs";
import { FirstRun } from "./screens/FirstRun";
import { StatusView } from "./screens/StatusView";

function Screen() {
  const t = useT();
  const { view, info, retry } = useDaemon();
  const link = view?.link ?? "connecting";
  const barKey = `bar.${link}` as const;
  return (
    <ToastProvider closeLabel={t("toast.close")}>
      <header>
        <h1>{t("app.title")}</h1>
        <AppMenu info={info} daemonVersion={view?.daemon_version ?? null} />
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
    </ToastProvider>
  );
}

/** `lang` is the initial choice (the tests give it); without it the language follows the system. */
export function App({ lang }: { lang?: LangChoice }) {
  return (
    <Preferences lang={lang}>
      <Screen />
    </Preferences>
  );
}
