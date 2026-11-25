import { describe, expect, it } from "vitest";
import { messages } from "./messages";

describe("messages", () => {
  it("every_key_exists_in_both_languages", () => {
    expect(Object.keys(messages.it).sort()).toEqual(Object.keys(messages.en).sort());
  });

  it("has_no_empty_text", () => {
    for (const [lang, table] of Object.entries(messages)) {
      for (const [key, text] of Object.entries(table)) {
        expect(text.trim(), `${lang}:${key}`).not.toBe("");
      }
    }
  });
});
