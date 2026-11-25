import { useState } from "react";
import { announce } from "./app/announce";
import type { SortOrder } from "./app/uiState";
import { useDaemon } from "./daemon/store";
import type { ThemeChoice } from "./design/theme";
import { useLang } from "./i18n";
import { useImportActions } from "./import/store";
import { Preferences, type LangChoice } from "./prefs";
import { displayOrder, useRubrica } from "./rubrica/store";
import { Shell } from "./shell/Shell";

function Screen({ initialSort }: { initialSort: SortOrder }) {
  const { view, info, retry } = useDaemon();
  const lang = useLang();
  const [sort, setSort] = useState<SortOrder>(initialSort);
  // The profile that takes the place of one that goes away is the next in the order the list shows.
  const rubrica = useRubrica((list) => displayOrder(list, sort, lang));
  const importer = useImportActions(rubrica.reload);
  return <Shell view={view} info={info} onRetry={retry} announce={announce} rubrica={rubrica} importer={importer} sort={sort} onSortChange={setSort} />;
}

/** `lang`, `theme` and `sort` are the initial choices (the saved ones, read by `main.tsx`; the tests give them); without them the app follows the system and sorts by name. */
export function App({ lang, theme, sort = "name" }: { lang?: LangChoice; theme?: ThemeChoice; sort?: SortOrder }) {
  return (
    <Preferences lang={lang} theme={theme}>
      <Screen initialSort={sort} />
    </Preferences>
  );
}
