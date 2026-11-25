// Helpers shared by the level 3 tests: the WebDriver smoke test (`*.e2e.ts`, lab/tests/gui-smoke.sh) and the screen
// reader checks (`*.a11y.ts`, lab/tests/gui-a11y.sh). Nothing here asks the daemon to connect.
import { execFile, execFileSync } from "node:child_process";
import { appendFileSync, existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { remote } from "webdriverio";

export type Browser = Awaited<ReturnType<typeof remote>>;

export function env(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is not set: run these tests through lab/tests/gui-smoke.sh or lab/tests/gui-a11y.sh`);
  return value;
}

/** The daemon is part of the scenario: the tests start and stop it from Node, not from the webview. */
export const daemon = (action: "start" | "stop") => execFileSync(`${env("GALLERIA_SMOKE_BIN")}/daemon-${action}`, { stdio: "inherit" });

export const TEN_SECONDS = 10_000;

let current: Browser | undefined;

/** `state.toml` of the app under test: gui-lib.sh points XDG_STATE_HOME into the temporary directory of the run. */
export const stateFile = (): string => join(env("XDG_STATE_HOME"), "galleria", "state.toml");

/** What the app wrote in `state.toml`, or an empty string while it has written nothing. */
export const readState = (): string => (existsSync(stateFile()) ? readFileSync(stateFile(), "utf8") : "");

/**
 * Starts the app. Every test starts from an empty `state.toml` and from the seeded rubrica, so what one test chose
 * (Italian, a theme) or did to the profiles does not leak into the next; `keepState` is for the test that restarts the
 * app on purpose and keeps both. `rubrica: "empty"` starts from a store with no profile.
 */
export async function openApp(keepState = false, rubrica: "seeded" | "empty" = "seeded"): Promise<Browser> {
  if (!keepState) {
    rmSync(stateFile(), { force: true });
    seedRubrica(rubrica);
  }
  current = await remote({
    hostname: "127.0.0.1",
    port: Number(env("GALLERIA_SMOKE_DRIVER_PORT")),
    logLevel: "error",
    capabilities: { "tauri:options": { application: env("GALLERIA_SMOKE_APP") } } as WebdriverIO.Capabilities,
  });
  return current;
}

export async function closeApp(): Promise<void> {
  await current?.deleteSession().catch(() => undefined);
  current = undefined;
}

export async function bodyText(b: Browser): Promise<string> {
  return b.$("body").getText();
}

export async function waitForBody(b: Browser, pattern: RegExp | string, timeoutMsg: string): Promise<void> {
  await b.waitUntil(
    async () => {
      const text = await bodyText(b);
      return typeof pattern === "string" ? text.includes(pattern) : pattern.test(text);
    },
    { timeout: TEN_SECONDS, interval: 250, timeoutMsg },
  );
}

// ---- Screen reader (lab/tests/gui-a11y.sh) -------------------------------------------------------------------------

export interface AtNode {
  /** Position in the dump: the descendants of a node are the nodes after it with a larger depth. */
  index: number;
  depth: number;
  role: string;
  name: string;
  states: Set<string>;
  description: string;
  relations: string[];
}

/** The accessibility tree of the Galleria window, read over AT-SPI by lab/tests/atspi-dump.py. */
export function atspiTree(): AtNode[] {
  const out = execFileSync("python3", [env("GALLERIA_A11Y_DUMP")], { encoding: "utf8" });
  return out
    .split("\n")
    .filter((line) => line.length > 0)
    .map((line, index) => {
      const [depth, role, name, states, description, relations] = line.split("\t");
      return {
        index,
        depth: Number(depth),
        role: role ?? "",
        name: name ?? "",
        states: new Set((states ?? "").split(",").filter(Boolean)),
        description: description ?? "",
        relations: (relations ?? "").split(",").filter(Boolean),
      };
    });
}

export interface SaveWindowAnswer {
  /** The name the window proposed in its name field. */
  offered: string;
  pressed: "Save" | "Cancel";
  /** The window was gone a moment after the button was pressed. */
  closed: boolean;
}

/**
 * The GTK «Save» window of «Export» is opened by Rust, not by the page, so WebDriver cannot see it: it is driven over
 * AT-SPI by lab/tests/atspi-save-window.py. `save` puts a full path in its name field and activates «Save» (the folder
 * must be there); `cancel` activates «Cancel». Throws when the window never appears. It does not block the event loop
 * (`spawnSync` would): the key that opened the window is still waiting for its answer on it.
 */
export async function answerSaveWindow(answer: { save: string } | "cancel"): Promise<SaveWindowAnswer> {
  const args = answer === "cancel" ? ["cancel"] : ["save", answer.save];
  const done = await new Promise<{ status: number | null; out: string }>((resolve) => {
    execFile("python3", [env("GALLERIA_A11Y_SAVE_WINDOW"), ...args], { encoding: "utf8" }, (error, stdout, stderr) =>
      resolve({ status: error ? ((error.code as number | undefined) ?? 1) : 0, out: `${stdout}${stderr}` }),
    );
  });
  if (done.status !== 0) throw new Error(`the Save window could not be answered (${done.status}): ${done.out}`);
  return JSON.parse(done.out) as SaveWindowAnswer;
}

export function descendants(tree: AtNode[], root: AtNode): AtNode[] {
  const out: AtNode[] = [];
  for (let i = root.index + 1; i < tree.length && tree[i].depth > root.depth; i++) out.push(tree[i]);
  return out;
}

export const findNode = (tree: AtNode[], role: string, name: string | RegExp): AtNode | undefined =>
  tree.find((n) => n.role === role && (typeof name === "string" ? n.name === name : name.test(n.name)));

export const focusedNode = (tree: AtNode[]): AtNode | undefined => [...tree].reverse().find((n) => n.states.has("focused") && n.role !== "frame");

export async function waitForTree(predicate: (tree: AtNode[]) => boolean, what: string, timeout = TEN_SECONDS): Promise<AtNode[]> {
  const deadline = Date.now() + timeout;
  let tree = atspiTree();
  while (!predicate(tree)) {
    if (Date.now() > deadline) throw new Error(`the accessibility tree never showed: ${what}\n${tree.map(formatNode).join("\n")}`);
    await new Promise((resolve) => setTimeout(resolve, 300));
    tree = atspiTree();
  }
  return tree;
}

export const formatNode = (n: AtNode): string =>
  `${"  ".repeat(n.depth)}${n.role} «${n.name}» [${[...n.states].join(",")}]${n.description ? ` description=«${n.description}»` : ""}${n.relations.length ? ` rel=${n.relations.join(",")}` : ""}`;

const SPEECH = /SPEECH OUTPUT: (.*) \{'established'/;

function spoken(): string[] {
  const log = env("GALLERIA_A11Y_ORCA_LOG");
  if (!existsSync(log)) return [];
  return readFileSync(log, "utf8")
    .split("\n")
    .flatMap((line) => {
      const text = SPEECH.exec(line)?.[1];
      return text === undefined ? [] : [text.replace(/^(['"])(.*)\1$/s, "$2")];
    });
}

/** Everything Orca has said so far counts as before the mark; `speechSince(mark)` is what it said after. */
export const speechMark = (): number => spoken().length;
export const speechSince = (mark: number): string[] => spoken().slice(mark);

/** Waits until something Orca said after `mark` matches, and returns everything it said since. */
export async function waitForSpeech(mark: number, pattern: RegExp | string, timeout = TEN_SECONDS): Promise<string[]> {
  const matches = (s: string) => (typeof pattern === "string" ? s.includes(pattern) : pattern.test(s));
  const deadline = Date.now() + timeout;
  for (;;) {
    const said = speechSince(mark);
    if (said.some(matches)) return said;
    if (Date.now() > deadline) throw new Error(`Orca never said ${String(pattern)}; it said: ${JSON.stringify(said)}`);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
}

/** Appends what a check saw to `transcript.txt`, which is copied into the outcome document. */
export function record(label: string, speech: string[], tree?: AtNode[]): void {
  const out = process.env.GALLERIA_A11Y_OUT;
  if (!out) return;
  const lines = [`## ${label}`, ...speech.map((s) => `SPEECH OUTPUT: ${s}`), ...(tree ? ["tree:", ...tree.map(formatNode)] : []), ""];
  appendFileSync(`${out}/transcript.txt`, `${lines.join("\n")}\n`);
}

/** The lines of Orca's own debug file that match `pattern` (it also holds the `EVENT MANAGER` lines, one per event). */
export function orcaDebugLines(pattern: RegExp): string[] {
  const log = env("GALLERIA_A11Y_ORCA_LOG");
  if (!existsSync(log)) return [];
  return readFileSync(log, "utf8")
    .split("\n")
    .filter((line) => pattern.test(line));
}

/**
 * Writes the made-up rubrica of `crates/galleria-gui/examples/seed-rubrica.rs` into the temporary config directory of the
 * run, replacing whatever was there: five profiles, three groups, two favourites (`"empty"`: a store with no profile).
 */
export function seedRubrica(kind: "seeded" | "empty" = "seeded"): void {
  execFileSync(`${env("GALLERIA_SMOKE_BIN")}/rubrica-seed`, kind === "empty" ? ["--empty"] : [], { stdio: "inherit" });
}
