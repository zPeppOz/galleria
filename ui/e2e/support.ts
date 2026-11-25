// Helpers shared by the level 3 tests: the WebDriver smoke test (`*.e2e.ts`, lab/tests/gui-smoke.sh) and the screen
// reader checks (`*.a11y.ts`, lab/tests/gui-a11y.sh). Nothing here asks the daemon to connect.
import { execFileSync } from "node:child_process";
import { appendFileSync, existsSync, readFileSync } from "node:fs";
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

export async function openApp(): Promise<Browser> {
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
