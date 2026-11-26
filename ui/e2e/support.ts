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
 * app on purpose and keeps both. `rubrica: "empty"` starts from a store with no profile. `args` are the arguments of the
 * process, as a file manager gives them: a file to open (S30: `tauri-driver` hands `tauri:options.args` to the app).
 */
export async function openApp(keepState = false, rubrica: "seeded" | "empty" = "seeded", args: string[] = []): Promise<Browser> {
  if (!keepState) {
    rmSync(stateFile(), { force: true });
    seedRubrica(rubrica);
  }
  current = await remote({
    hostname: "127.0.0.1",
    port: Number(env("GALLERIA_SMOKE_DRIVER_PORT")),
    logLevel: "error",
    capabilities: { "tauri:options": { application: env("GALLERIA_SMOKE_APP"), ...(args.length > 0 ? { args } : {}) } } as WebdriverIO.Capabilities,
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

export interface FileWindowAnswer {
  /** What the field of the name (or of the path) held when the probe found it: in «Save», the name the window proposed. */
  offered: string;
  pressed: "Save" | "Open" | "Cancel";
  /** The window was gone a moment after the button was pressed. */
  closed: boolean;
}

export interface FileWindowDump {
  entries: string[];
  buttons: string[];
  /** The combo box of the filters and what it offers. */
  filters: string[];
}

/** Runs the probe of the GTK window of Rust and answers with what it printed. It does not block the event loop (`spawnSync` would): the key that opened the window is still waiting for its answer on it. */
async function fileWindow(args: string[]): Promise<string> {
  const done = await new Promise<{ status: number | null; out: string }>((resolve) => {
    execFile("python3", [env("GALLERIA_A11Y_FILE_WINDOW"), ...args], { encoding: "utf8" }, (error, stdout, stderr) =>
      resolve({ status: error ? ((error.code as number | undefined) ?? 1) : 0, out: `${stdout}${stderr}` }),
    );
  });
  if (done.status !== 0) throw new Error(`the file window could not be answered (${done.status}): ${done.out}`);
  return done.out;
}

/**
 * The GTK windows «Save» (of «Export») and «Open» (of «Import») are opened by Rust, not by the page, so WebDriver cannot
 * see them: they are driven over AT-SPI by lab/tests/atspi-file-window.py. `save` puts a full path in the name field and
 * activates «Save» (the folder must be there); `open` brings up the field of the path, puts a full path in it and
 * activates «Open»; `cancel` activates «Cancel». Throws when the window never appears.
 */
export async function answerFileWindow(answer: { save: string } | { open: string } | "cancel"): Promise<FileWindowAnswer> {
  const args = answer === "cancel" ? ["cancel"] : "save" in answer ? ["save", answer.save] : ["open", answer.open];
  return JSON.parse(await fileWindow(args)) as FileWindowAnswer;
}

/** What the window has: the fields, the buttons and the filters, without touching it. */
export async function dumpFileWindow(): Promise<FileWindowDump> {
  return JSON.parse(await fileWindow(["dump"])) as FileWindowDump;
}

/** What the source of a real drag reports (lab/tests/gtk-drag-source.py): where it started, where it let go, and whether a window took the file. */
export interface DragResult {
  source: [number, number];
  target: [number, number];
  ended: boolean;
  dropped: boolean;
}

/**
 * Drags a file onto the window of Galleria with a real drag: a small GTK window offers the file as `text/uri-list`, the
 * mouse is pressed on it, moved over the window and released there, over AT-SPI (S28). With `cancel` the button is
 * released in the corner of the screen, where nothing takes a drop. It does not block the event loop.
 */
export async function dragFile(path: string, options: { cancel?: boolean } = {}): Promise<DragResult> {
  const done = await new Promise<{ status: number | null; out: string }>((resolve) => {
    execFile("python3", [env("GALLERIA_A11Y_DRAG_SOURCE"), path, ...(options.cancel ? ["--cancel"] : [])], { encoding: "utf8" }, (error, stdout, stderr) =>
      resolve({ status: error ? ((error.code as number | undefined) ?? 1) : 0, out: `${stdout}${stderr}` }),
    );
  });
  if (done.status !== 0) throw new Error(`the drag did not end (${done.status}): ${done.out}`);
  return JSON.parse(done.out.trim().split("\n").pop() ?? "{}") as DragResult;
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

/**
 * Writes the files an import is tried on (`.ovpn`, `.conf` and the files they refer to, the malicious profile of the
 * laboratory) into a folder of the run and answers with their paths by name: `inline`, `separate`, `malicious`, `dns`,
 * `outside`, `office`, `server`. The folder is outside the rubrica, so seeding it again does not touch them.
 */
export function importFixtures(): Record<string, string> {
  const out = execFileSync(`${env("GALLERIA_SMOKE_BIN")}/import-fixtures`, [], { encoding: "utf8" });
  return Object.fromEntries(
    out
      .split("\n")
      .filter((line) => line.includes("="))
      .map((line) => [line.slice(0, line.indexOf("=")), line.slice(line.indexOf("=") + 1)]),
  );
}
