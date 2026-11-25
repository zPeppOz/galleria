import { useDaemon } from "./daemon/store";
import type { ThemeChoice } from "./design/theme";
import { Preferences, type LangChoice } from "./prefs";
import { Shell } from "./shell/Shell";

function Screen() {
  const { view, info, retry } = useDaemon();
  return <Shell view={view} info={info} onRetry={retry} />;
}

/** `lang` and `theme` are the initial choices (the saved ones, read by `main.tsx`; the tests give them); without them the app follows the system. */
export function App({ lang, theme }: { lang?: LangChoice; theme?: ThemeChoice }) {
  return (
    <Preferences lang={lang} theme={theme}>
      <Screen />
    </Preferences>
  );
}
