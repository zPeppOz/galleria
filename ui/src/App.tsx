import { useDaemon } from "./daemon/store";
import { Preferences, type LangChoice } from "./prefs";
import { Shell } from "./shell/Shell";

function Screen() {
  const { view, info, retry } = useDaemon();
  return <Shell view={view} info={info} onRetry={retry} />;
}

/** `lang` is the initial choice (the tests give it); without it the language follows the system. */
export function App({ lang }: { lang?: LangChoice }) {
  return (
    <Preferences lang={lang}>
      <Screen />
    </Preferences>
  );
}
