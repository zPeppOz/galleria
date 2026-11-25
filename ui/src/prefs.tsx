import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { applyTheme, resolveTheme, systemPrefersLight, watchSystemTheme, type ThemeChoice } from "./design/theme";
import { detectLang, I18n, type Lang } from "./i18n";

export type LangChoice = "system" | Lang;

interface Prefs {
  /** What the user chose: `system` follows the system. */
  lang: LangChoice;
  theme: ThemeChoice;
  setLang: (choice: LangChoice) => void;
  setTheme: (choice: ThemeChoice) => void;
}

const PrefsContext = createContext<Prefs | null>(null);

/**
 * Language and theme. In B2 they live in memory only (Q25): they are lost when the window closes, and B3 saves them.
 * Renders the language provider and writes `lang` and `data-theme` on the root element.
 */
export function Preferences({ lang: initialLang = "system", theme: initialTheme = "system", children }: { lang?: LangChoice; theme?: ThemeChoice; children: ReactNode }) {
  const [lang, setLang] = useState<LangChoice>(initialLang);
  const [theme, setTheme] = useState<ThemeChoice>(initialTheme);
  const [light, setLight] = useState(systemPrefersLight);
  const language = lang === "system" ? detectLang() : lang;

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);
  useEffect(() => watchSystemTheme(setLight), []);
  useEffect(() => {
    applyTheme(document.documentElement, resolveTheme(theme, light));
  }, [theme, light]);

  const value = useMemo<Prefs>(() => ({ lang, theme, setLang, setTheme }), [lang, theme]);
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
