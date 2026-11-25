import { createContext, useContext, useMemo, type ReactNode } from "react";
import en from "./en.json";
import it from "./it.json";

export type Lang = "en" | "it";
export type MessageKey = keyof typeof en;
export type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string;

// Typed so that a key missing from the Italian catalogue does not compile.
const catalogues: Record<Lang, Record<MessageKey, string>> = { en, it };

export const hasKey = (key: string): key is MessageKey => key in en;

/** The language of the system when it is Italian; English otherwise, which is also the fallback. */
export const detectLang = (): Lang => (navigator.language.toLowerCase().startsWith("it") ? "it" : "en");

const LangContext = createContext<Lang>("en");

/** Gives the language to the components below. Nothing is translated before it is drawn, so a change shows at once. */
export function I18n({ lang, children }: { lang: Lang; children: ReactNode }) {
  return <LangContext.Provider value={lang}>{children}</LangContext.Provider>;
}

export function useT(): Translate {
  const lang = useContext(LangContext);
  return useMemo<Translate>(
    () => (key, vars = {}) => catalogues[lang][key].replace(/\{(\w+)\}/g, (_, name: string) => String(vars[name] ?? "")),
    [lang],
  );
}

/** D22: the catalogue text for the code; for a code the catalogue does not know, the English message of the daemon. */
export function errorText(t: Translate, code: string, fallback: string): string {
  const key = `error.${code}`;
  return hasKey(key) ? t(key) : fallback;
}

/** The language the window is in. */
export const useLang = (): Lang => useContext(LangContext);
