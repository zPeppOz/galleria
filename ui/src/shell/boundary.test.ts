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

  it("the_page_never_listens_to_the_drag_events_of_tauri", () => {
    // Tauri also sends the paths of a dragged file to the page, as events, to whoever listens (Q38). The page does not: the
    // paths stay in Rust, and the page only hears the two events of Galleria, which carry none.
    const sourcesOfThePage = webviewSources(SRC);
    for (const { file, text } of sourcesOfThePage) {
      for (const forbidden of ["tauri://", "onDragDropEvent", "getCurrentWebview", "getCurrentWindow", "TauriEvent"]) {
        expect(text.includes(forbidden), `${file} names ${forbidden}`).toBe(false);
      }
      for (const match of text.matchAll(/\blisten(?:<[^>(]*>)?\(\s*([^,)]*)/g)) {
        expect(match[1], `${file}: listen(${match[1]}`).toMatch(/^"galleria:[a-z-]+"$/);
      }
    }
    // the events the page does listen to are the ones of Galleria, and the check above has something to look at
    const heard = sourcesOfThePage.flatMap(({ text }) => [...text.matchAll(/\blisten(?:<[^>(]*>)?\(\s*"([^"]*)"/g)].map((m) => m[1]));
    expect(heard).toContain("galleria:open-file");
    expect(heard).toContain("galleria:drop-hover");
  });

  it("the_credentials_dialog_keeps_no_value_in_state", () => {
    // Step 1 of «Il viaggio di una credenziale»: what is typed is in the `<input>` and nowhere else. The file has no state, no
    // store and no log, and the answer goes through one function that is called from one place.
    const dialog = readFileSync(join(SRC, "connection", "CredentialsDialog.tsx"), "utf8");
    for (const forbidden of [/\buseState\b/, /\buseReducer\b/, /\bcreateContext\b/, /\buseContext\b/, /\bconsole\./, /\blocalStorage\b/, /\bsessionStorage\b/, /\bvalue=\{/, /\bdefaultValue\b/, /\bonChange\b/]) {
      expect(forbidden.test(dialog), `CredentialsDialog.tsx matches ${String(forbidden)}`).toBe(false);
    }
    const sourcesOfThePage = webviewSources(SRC);
    expect(sourcesOfThePage.filter(({ text }) => text.includes('"provide_input"')).map(({ file }) => file)).toEqual([join("daemon", "commands.ts")]);
    // and the function that invokes it is called from the connection only
    expect(sourcesOfThePage.filter(({ file, text }) => file !== join("daemon", "commands.ts") && /\bprovideInput\(/.test(text)).map(({ file }) => file)).toEqual([join("connection", "ConnectionProvider.tsx")]);
  });

  it("no_sample_data_is_left", () => {
    expect(existsSync(join(SHELL, `${"de"}mo-data.ts`))).toBe(false);
    expect(webviewSources(SRC).filter(({ text }) => /Sample data|Dati di esempio/.test(text)).map(({ file }) => file)).toEqual([]);
  });

  it("the_css_of_b1_is_gone", () => {
    expect(existsSync(join(SHELL, "..", "styles.css"))).toBe(false);
  });
});
