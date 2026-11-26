// Helpers shared by the level 3 tests: the WebDriver smoke test (`*.e2e.ts`, lab/tests/gui-smoke.sh) and the screen
// reader checks (`*.a11y.ts`, lab/tests/gui-a11y.sh). Nothing here asks the daemon to connect.
import { execFile, execFileSync, spawn, spawnSync, type ChildProcess } from "node:child_process";
import { connect } from "node:net";
import { appendFileSync, existsSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { remote } from "webdriverio";

export type Browser = Awaited<ReturnType<typeof remote>>;

export function env(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is not set: run these tests through lab/tests/gui-smoke.sh or lab/tests/gui-a11y.sh`);
  return value;
}

/** The daemon is part of the scenario: the tests start and stop it from Node, not from the webview. `scenario` is a name of `tests/support/scenarios.rs` (what the fake backend does at each connect); it counts only when the daemon is started. */
export const daemon = (action: "start" | "stop", scenario?: string) => execFileSync(`${env("GALLERIA_SMOKE_BIN")}/daemon-${action}`, scenario ? [scenario] : [], { stdio: "inherit" });

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
export async function openApp(keepState = false, rubrica: "seeded" | "empty" = "seeded", args: string[] = [], keyring: "system" | "none" | "no-indicator" = "system"): Promise<Browser> {
  if (!keepState) {
    rmSync(stateFile(), { force: true });
    seedRubrica(rubrica);
  }
  const port = process.env.GALLERIA_DRIVER_PER_SESSION ? await startPrivateDriver() : Number(env("GALLERIA_SMOKE_DRIVER_PORT"));
  try {
    current = await remote({
      hostname: "127.0.0.1",
      port,
      logLevel: "error",
      capabilities: { "tauri:options": { application: env(keyring === "none" ? "GALLERIA_SMOKE_APP_NO_KEYRING" : keyring === "no-indicator" ? "GALLERIA_SMOKE_APP_NO_INDICATOR" : "GALLERIA_SMOKE_APP"), ...(args.length > 0 ? { args } : {}) } } as WebdriverIO.Capabilities,
    });
  } catch (failure) {
    await stopPrivateDriver();
    throw failure;
  }
  return current;
}

export async function closeApp(): Promise<void> {
  const closing = current?.deleteSession().catch(() => undefined);
  current = undefined;
  // With a driver of its own the session is ended for good in the next lines, so a driver that hangs on this call is not waited for
  if (privateDriver) await Promise.race([closing, new Promise((resolve) => setTimeout(resolve, TEN_SECONDS))]);
  else await closing;
  await stopPrivateDriver();
}

// ---- A driver of its own for every session (GALLERIA_DRIVER_PER_SESSION, set by lab/tests/gui-a11y.sh) --------------------
// One `tauri-driver` and one `WebKitWebDriver` serve every session of a run when the script starts them (the smoke test). If
// that driver dies, every later test finds the port refused and fails with it (the cascade of `UND_ERR_SOCKET` and «connection
// refused» seen in the screen reader check). With this variable each session starts its own pair and the end of the session
// ends it: a driver that dies fails the test that was using it and no other.

let privateDriver: ChildProcess | undefined;
let nextPort = 4500;

async function startPrivateDriver(): Promise<number> {
  await stopPrivateDriver();
  const port = nextPort;
  nextPort += 2;
  const child = spawn(env("GALLERIA_SMOKE_DRIVER"), ["--port", String(port), "--native-port", String(port + 1), "--native-driver", env("GALLERIA_NATIVE_DRIVER_WRAPPER")], { stdio: "inherit" });
  privateDriver = child;
  let exited = false;
  child.once("exit", () => (exited = true));
  const deadline = Date.now() + TEN_SECONDS;
  while (!(await listening(port))) {
    if (exited) throw new Error(`tauri-driver exited before it listened on ${port}`);
    if (Date.now() > deadline) throw new Error(`tauri-driver did not listen on ${port} in ${TEN_SECONDS} ms`);
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  return port;
}

const listening = (port: number): Promise<boolean> =>
  new Promise((resolve) => {
    const socket = connect({ host: "127.0.0.1", port });
    socket.once("connect", () => (socket.destroy(), resolve(true)));
    socket.once("error", () => (socket.destroy(), resolve(false)));
  });

/** Ends the driver of the session and waits until it is gone, and with it the window it opened if the driver left it behind. */
async function stopPrivateDriver(): Promise<void> {
  const child = privateDriver;
  privateDriver = undefined;
  if (!child) return;
  if (child.exitCode === null && child.signalCode === null) {
    const gone = new Promise<void>((resolve) => child.once("exit", () => resolve()));
    child.kill("SIGTERM");
    await Promise.race([gone, new Promise((resolve) => setTimeout(resolve, 5000))]);
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  }
  // A driver that died (or was ended before it had closed its session) leaves the window running, and the window holds the
  // lock of the single instance (plan B4, T5): every app that the next tests start would hand its arguments over to it and
  // exit, and each of those tests would wait for a session that never comes. The run has no other galleria-gui.
  spawnSync("pkill", ["-KILL", "-x", "galleria-gui"]);
  const deadline = Date.now() + TEN_SECONDS;
  while (spawnSync("pgrep", ["-x", "galleria-gui"]).status === 0 && Date.now() < deadline) await new Promise((resolve) => setTimeout(resolve, 50));
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

// ---- Helpers of the tests that connect (ui/e2e/lab-connect.e2e.ts) ---------------------------------------------------

export const waitFor = (b: Browser, check: () => Promise<boolean>, timeoutMsg: string, timeout = TEN_SECONDS) => b.waitUntil(check, { timeout, interval: 250, timeoutMsg });

/** The text of a cell of the strip of the profile that is open (`strip.state`, `strip.ip`, …). */
export const cell = async (b: Browser, name: string): Promise<string> => (await b.$(`[data-cell="${name}"] dd`).getText()).trim();

/** The state the bar of the window says. */
export const barState = async (b: Browser): Promise<string> => (await b.$(".status-bar-state").getText()).trim();

/** Chooses the row of the list whose name starts with `name` and waits for its detail. */
export async function choose(b: Browser, name: string): Promise<void> {
  const row = await b.waitUntil(
    async () => {
      for (const el of await b.$$('[role="listbox"] [role="option"]')) if ((await el.getText()).trim().startsWith(name)) return el;
      return false;
    },
    { timeout: TEN_SECONDS, interval: 250, timeoutMsg: `the list has no row «${name}»` },
  );
  await row.click();
  await waitFor(b, async () => (await b.$("h1").getText()) === name, `the detail never showed «${name}»`);
}

/** The id of the profile of this name: the file of the rubrica is named after it. */
export function profileId(name: string): string {
  const dir = join(env("XDG_CONFIG_HOME"), "galleria", "profiles");
  const file = readdirSync(dir).find((entry) => readFileSync(join(dir, entry), "utf8").includes(`name = "${name}"`));
  if (!file) throw new Error(`no profile named ${name} on the disk`);
  return file.replace(/\.toml$/, "");
}

/** What the keyring holds for one key of a profile (empty when it holds nothing): read with `secret-tool`, as a person would. */
export const keyringLookup = (id: string, key: string): string => spawnSync("secret-tool", ["lookup", "service", "galleria", "profile", id, "key", key], { encoding: "utf8" }).stdout;

/** Every item of the keyring of a profile, with its secret, as text. */
export const keyringItems = (id: string): string => `${spawnSync("secret-tool", ["search", "--all", "service", "galleria", "profile", id], { encoding: "utf8" }).stdout}`;

// ---- The tray and the window seen from outside (plan B6): lab/tests/tray-probe.py, lab/tests/x11-window.py ---------------------------
// The tray is an AppIndicator: the desktop draws it, and under Xvfb there is no desktop. The probe stands in for the one that shows the
// tray (the `StatusNotifierWatcher`) and reads and clicks the menu from the session bus; the other script reads whether the window is
// shown and sends it the request of closing that a window manager would. A test that wants a tray starts the watcher; the others have
// none, so for them a closed window is a quit (Q53) like it was before B6.

export interface TrayEntryView { id: number; text: string; enabled: boolean }

const python = (script: string, args: string[], options: { allowFailure?: boolean } = {}): string => {
  const done = spawnSync("python3", [env(script), ...args], { encoding: "utf8" });
  if (done.status !== 0 && !options.allowFailure) throw new Error(`${script} ${args.join(" ")} (${done.status}): ${done.stdout}${done.stderr}`);
  return `${done.stdout}`.trim();
};

let watcher: ChildProcess | undefined;

/** The folder where the probe keeps what it saw: the same for the watcher and for the commands (they are separate processes). */
const probeDir = (): string => join(env("XDG_RUNTIME_DIR"), "tray-probe");

/** Starts the watcher: from now on an app that makes a tray finds someone who shows it. Waits until it owns its name on the bus. */
export async function startTrayWatcher(): Promise<void> {
  stopTrayWatcher();
  process.env.TRAY_PROBE_DIR = probeDir();
  watcher = spawn("python3", [env("GALLERIA_TRAY_PROBE"), "watch"], { stdio: "ignore" });
  const deadline = Date.now() + TEN_SECONDS;
  while (!existsSync(join(probeDir(), "items.json"))) {
    if (Date.now() > deadline) throw new Error("the tray watcher did not start");
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  // `items.json` is written before the name is asked for: one more moment, so that the app that comes next finds the name taken
  await new Promise((resolve) => setTimeout(resolve, 300));
}

export function stopTrayWatcher(): void {
  watcher?.kill("SIGKILL");
  watcher = undefined;
  rmSync(probeDir(), { recursive: true, force: true });
}

/** The entries of the menu of the tray as the desktop would draw it (separators left out). */
export const trayMenu = (): TrayEntryView[] => JSON.parse(python("GALLERIA_TRAY_PROBE", ["layout"])) as TrayEntryView[];
export const trayTexts = (): string[] => trayMenu().map((entry) => entry.text);
export const trayClick = (text: string): void => void python("GALLERIA_TRAY_PROBE", ["click", text]);
export const trayIcon = (): string => python("GALLERIA_TRAY_PROBE", ["icon"]);
export const trayRegistered = (seconds = 10): boolean => spawnSync("python3", [env("GALLERIA_TRAY_PROBE"), "wait-item", String(seconds)]).status === 0;

/** Waits until `read()` gives what `check` accepts; says what it saw last when it never does. */
export async function until<T>(read: () => T, check: (value: T) => boolean, what: string, timeout = TEN_SECONDS): Promise<T> {
  const deadline = Date.now() + timeout;
  let last: T | undefined;
  let failure: unknown;
  for (;;) {
    try {
      last = read();
      failure = undefined;
      if (check(last)) return last;
    } catch (error) {
      failure = error;
    }
    if (Date.now() > deadline) throw new Error(`${what}; last seen: ${failure ? String(failure) : JSON.stringify(last)}`);
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
}

/** `true` when the window is shown, `false` when it is hidden, `none` when there is no window. */
export const windowState = (): string => python("GALLERIA_X11_WINDOW", ["mapped"]);

/** The request of closing that the «X» of a title bar would send. */
export const requestClose = (): void => void python("GALLERIA_X11_WINDOW", ["close"]);

/** The app is running (a process of that name exists). */
export const appRunning = (): boolean => spawnSync("pgrep", ["-x", "galleria-gui"]).status === 0;
