/// <reference types="node" />
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const SHELL = import.meta.dirname;
const SRC = join(SHELL, "..");
const CAPABILITY = join(SRC, "..", "..", "crates", "galleria-gui", "capabilities", "main.json");
const sources = readdirSync(SHELL).filter((name) => /\.(tsx?|css)$/.test(name) && !/\.test\./.test(name));

/** Every source of the webview except the tests, with its text. */
function webviewSources(dir: string): { file: string; text: string }[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return webviewSources(path);
    const file = path.slice(SRC.length + 1);
    return /\.tsx?$/.test(name) && !/\.test\./.test(name) && !file.startsWith("test/") ? [{ file, text: readFileSync(path, "utf8") }] : [];
  });
}

/** The commands the main window may call, from its capability: `allow-profiles-list` is `profiles_list`. */
function allowedCommands(): string[] {
  const permissions = (JSON.parse(readFileSync(CAPABILITY, "utf8")) as { permissions: string[] }).permissions;
  return permissions.filter((p) => !p.includes(":")).map((p) => p.replace(/^allow-/, "").replaceAll("-", "_"));
}

describe("shell sources", () => {
  it("the_shell_has_sources_to_check", () => {
    expect(sources.length).toBeGreaterThan(8);
  });

  it("the_shell_draws_what_it_is_given_and_asks_for_nothing", () => {
    // The data come in as props (the daemon link, the rubrica); a component of the shell never invokes a command.
    const offenders = sources.filter((name) => /invoke\(|@tauri-apps|daemon\/store|rubrica\/store"/.test(readFileSync(join(SHELL, name), "utf8").replace(/import type [^;]*;/g, "")));
    expect(offenders).toEqual([]);
  });

  it("shell_only_calls_the_listed_commands", () => {
    // Every command invoked anywhere in the webview is one the capability of the main window grants, and every granted
    // command is invoked from somewhere: a command nobody calls is a permission nobody needs.
    const invoked = new Set<string>();
    for (const { text } of webviewSources(SRC)) for (const match of text.matchAll(/\binvoke(?:<[^>(]*>)?\(\s*"([a-z_]+)"/g)) invoked.add(match[1]);
    const allowed = allowedCommands();
    expect(allowed.length).toBeGreaterThan(5);
    expect([...invoked].filter((name) => !allowed.includes(name)).sort()).toEqual([]);
    expect(allowed.filter((name) => !invoked.has(name)).sort()).toEqual([]);
  });

  it("no_sample_data_is_left", () => {
    expect(existsSync(join(SHELL, `${"de"}mo-data.ts`))).toBe(false);
    expect(webviewSources(SRC).filter(({ text }) => /Sample data|Dati di esempio/.test(text)).map(({ file }) => file)).toEqual([]);
  });

  it("the_css_of_b1_is_gone", () => {
    expect(existsSync(join(SHELL, "..", "styles.css"))).toBe(false);
  });
});
