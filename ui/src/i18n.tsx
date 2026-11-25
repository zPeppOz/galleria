import { createContext, useContext, type ReactNode } from "react";
import { messages, type Lang, type MessageKey } from "./messages";

export const detectLang = (): Lang => (navigator.language.toLowerCase().startsWith("it") ? "it" : "en");
const LangContext = createContext<Lang>("en");

/** Gives the language to the screens: the given one, else the system's, English as the fallback. */
export function I18n({ lang, children }: { lang?: Lang; children: ReactNode }) {
  return <LangContext.Provider value={lang ?? detectLang()}>{children}</LangContext.Provider>;
}

export type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string;
export function useT(): Translate {
  const lang = useContext(LangContext);
  return (key, vars = {}) => messages[lang][key].replace(/\{(\w+)\}/g, (_, name: string) => String(vars[name] ?? ""));
}
