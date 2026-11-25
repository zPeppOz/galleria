/// <reference types="node" />
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const SHELL = import.meta.dirname;
const sources = readdirSync(SHELL).filter((name) => /\.(tsx?|css)$/.test(name) && !/\.test\./.test(name));

describe("shell sources", () => {
  it("the_shell_has_sources_to_check", () => {
    expect(sources.length).toBeGreaterThan(8);
  });

  it("shell_never_reaches_the_daemon", () => {
    // The sample data never leave the page: no command, no event, no import of the Tauri API in the shell.
    const offenders = sources.filter((name) => /invoke\(|@tauri-apps|daemon\/store/.test(readFileSync(join(SHELL, name), "utf8")));
    expect(offenders).toEqual([]);
  });

  it("the_css_of_b1_is_gone", () => {
    expect(existsSync(join(SHELL, "..", "styles.css"))).toBe(false);
  });
});
