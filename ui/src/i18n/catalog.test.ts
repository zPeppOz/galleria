import { describe, expect, it } from "vitest";
import en from "./en.json";
import it_ from "./it.json";
import { errorText, hasKey, type MessageKey } from "./index";

const catalogues = { en, it: it_ } as Record<string, Record<string, string>>;
const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("catalogues", () => {
  it("every_key_exists_in_both_languages", () => {
    expect(Object.keys(catalogues.it).sort()).toEqual(Object.keys(catalogues.en).sort());
  });

  it("has_no_empty_text", () => {
    for (const [lang, table] of Object.entries(catalogues)) {
      for (const [key, text] of Object.entries(table)) {
        expect(text.trim(), `${lang}:${key}`).not.toBe("");
      }
    }
  });

  it("placeholders_match_between_languages", () => {
    for (const key of Object.keys(catalogues.en)) {
      expect(placeholders(catalogues.it[key]), key).toEqual(placeholders(catalogues.en[key]));
    }
  });

  it("has_key_tells_catalogue_keys_from_other_strings", () => {
    expect(hasKey("error.auth_failed")).toBe(true);
    expect(hasKey("error.brand_new_code")).toBe(false);
  });
});

describe("errorText", () => {
  const t = (key: MessageKey) => catalogues.it[key];

  it("unknown_error_code_falls_back_to_the_daemon_message", () => {
    expect(errorText(t, "brand_new_code", "Daemon said so")).toBe("Daemon said so");
  });

  it("known_error_code_is_translated", () => {
    expect(errorText(t, "auth_failed", "Authentication failed")).toBe(catalogues.it["error.auth_failed"]);
  });
});
