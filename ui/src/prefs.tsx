import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { saveUiPrefs } from "./app/uiState";
import { applyTheme, resolveTheme, systemPrefersLight, watchSystemTheme, type ThemeChoice } from "./design/theme";
import { detectLang, I18n, type Lang } from "./i18n";

export type LangChoice = "system" | Lang;

interface Prefs {
  /** What the user chose: `system` follows the system. */
  lang: LangChoice;
  theme: ThemeChoice;
  /**
   * The choice applies at once and is written to `state.toml` (D32). The promise rejects when the write fails: the
   * choice stays valid for the session, and whoever called says so.
   */
  setLang: (choice: LangChoice) => Promise<void>;
  setTheme: (choice: ThemeChoice) => Promise<void>;
}

const PrefsContext = createContext<Prefs | null>(null);

/**
 * Language and theme, saved by the Rust side (Q25, D32). The initial values are the saved ones, read before the first
 * render. Renders the language provider and writes `lang` and `data-theme` on the root element.
 */
export function Preferences({ lang: initialLang = "system", theme: initialTheme = "system", children }: { lang?: LangChoice; theme?: ThemeChoice; children: ReactNode }) {
  const [lang, setLangState] = useState<LangChoice>(initialLang);
  const [theme, setThemeState] = useState<ThemeChoice>(initialTheme);
  const [light, setLight] = useState(systemPrefersLight);
  const language = lang === "system" ? detectLang() : lang;

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);
  useEffect(() => watchSystemTheme(setLight), []);
  useEffect(() => {
    applyTheme(document.documentElement, resolveTheme(theme, light));
  }, [theme, light]);

  const setLang = useCallback((choice: LangChoice) => {
    setLangState(choice);
    return saveUiPrefs({ lang: choice });
  }, []);
  const setTheme = useCallback((choice: ThemeChoice) => {
    setThemeState(choice);
    return saveUiPrefs({ theme: choice });
  }, []);

  const value = useMemo<Prefs>(() => ({ lang, theme, setLang, setTheme }), [lang, theme, setLang, setTheme]);
  return (
    <PrefsContext.Provider value={value}>
      <I18n lang={language}>{children}</I18n>
    </PrefsContext.Provider>
  );
}

export function usePrefs(): Prefs {
  const prefs = useContext(PrefsContext);
  if (!prefs) throw new Error("usePrefs needs <Preferences> above it");
  return prefs;
}
