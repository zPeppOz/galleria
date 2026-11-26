// Plan B5, groups C and D: the connection under a screen reader (Q22 (a): Orca and the AT-SPI tree in the real window, run locally by
// lab/tests/gui-a11y.sh, i.e. `scripts/gui.sh a11y`). The daemon is the fake one, with a scenario. What Orca said goes into
// target/gui-a11y/transcript.txt.
//   K5a  «Connected to …» is said once, polite, by Rust · K5b  an error is said once, assertive
//   K5c  the window of the credentials is a dialog with the focus on its first field, and says nothing of its own
//   K5d  the question about a session of someone else is read, with the focus on «Cancel»
//   K5e  «Export log…» through the real «Save» window (GTK, driven over AT-SPI): the file is written, private, and the outcome is said once
//   K5f  a switch of profile: the notice of what was left and then the state, once each
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { answerFileWindow, atspiTree, bodyText, choose, closeApp, daemon, descendants, env, findNode, focusedNode, openApp, orcaDebugLines, record, speechMark, speechSince, TEN_SECONDS, waitForBody, waitForSpeech, waitForTree, type AtNode, type Browser } from "./support";

let b: Browser;

const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const waitFor = (check: () => Promise<boolean>, timeoutMsg: string) => b.waitUntil(check, { timeout: TEN_SECONDS, interval: 250, timeoutMsg });

async function openWith(scenario: string): Promise<void> {
  daemon("stop");
  daemon("start", scenario);
  b = await openApp();
  await waitForBody(b, "Connected to the service", "the shell never showed the link");
}

afterEach(closeApp);

describe("the log of a session under a screen reader", () => {
  beforeEach(async () => {
    await openWith("plain");
    await b.$("button=Connect").click();
    await waitFor(async () => (await b.$('[data-cell="strip.state"] dd').getText()).trim() === "Connected", "the profile never connected");
    await b.$("button=Log").click();
    await waitFor(async () => (await b.$$(".log-line")).length > 0, "the tab «Log» never showed a line");
  });

  it("K5e_the_log_is_exported_through_the_real_save_window", async () => {
    const dir = mkdtempSync(join(tmpdir(), "galleria-log-"));
    const target = join(dir, "session.txt");
    // the focus goes to the button and the key asks; the key is not awaited, because the command answers when the window of Rust closes
    await b.execute(() => [...document.querySelectorAll<HTMLElement>("button")].find((el) => /^Export log/.test(el.textContent ?? ""))?.focus());
    const mark = speechMark();
    void b.keys(["Enter"]).catch(() => undefined);
    const answer = await answerFileWindow({ save: target });
    expect(answer.offered, "the window proposes the name of P12").toMatch(/^galleria-log-\d{8}-\d{4}\.txt$/);
    expect(answer).toMatchObject({ pressed: "Save", closed: true });
    const DONE = "Log exported.";
    await waitForSpeech(mark, DONE, 5000);
    await settle(1500);
    const times = speechSince(mark).filter((line) => line.includes(DONE)).length;
    record("K5e the export of the log, saved through the real window", speechSince(mark), atspiTree());
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(DONE));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${DONE})`);
    // the file is private, holds the lines of the session in the format of `galleria logs`, and nothing else was left next to it
    expect(statSync(target).mode & 0o777).toBe(0o600);
    expect(readFileSync(target, "utf8")).toMatch(/^\d+ Info Daemon: .*connect/m);
    expect(readdirSync(dir)).toEqual(["session.txt"]);
    // the page never learns the place
    expect(await bodyText(b)).not.toContain(dir);
    // and Orca did not read the lines of the log as they came (the list is a log with its announcements off)
    expect(speechSince(mark).filter((line) => /Info Daemon|connect$/.test(line))).toEqual([]);
  });
});

// ---- The states, the credentials and the question about a session of someone else (plan B5, T11) -----------------------------
// Q27: Rust says the changes of state (polite; an error is assertive); the window of the credentials and the questions take the focus and
// say nothing of their own. The daemon is the fake one, with a scenario; the session of someone else is started by the CLI of another rubrica.

describe("the connection under a screen reader", () => {
  const announced = () => orcaDebugLines(/Dequeued object:announcement/);
  const said = (mark: number, text: string | RegExp) => speechSince(mark).filter((line) => (typeof text === "string" ? line.includes(text) : text.test(line)));

  it("K5a_connected_is_said_once_polite", async () => {
    await openWith("plain");
    const mark = speechMark();
    await b.$("button=Connect").click();
    const SAID = "Connected to Ufficio Napoli";
    await waitForSpeech(mark, SAID, 15_000);
    await settle(1500);
    record("K5a the state «connected», said by Rust", speechSince(mark), atspiTree());
    expect(said(mark, SAID), `Orca said it ${said(mark, SAID).length} times: ${JSON.stringify(speechSince(mark))}`).toHaveLength(1);
    // the window of Rust sent it as *polite*: `(1, 0, text)`; and the page was not the one that said it (it has no call for the states)
    const lines = announced().filter((line) => line.includes(SAID));
    expect(lines.length, "Orca dequeued an object:announcement carrying the state").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${SAID})`);
    // «Connecting» was not announced: nothing but the one phrase
    expect(announced().filter((line) => /Connecting/.test(line))).toEqual([]);
  });

  it("K5b_an_error_is_said_assertive", async () => {
    await openWith("auth-failed");
    await b.$("button=Connect").click();
    // the scenario asks for the credentials and then refuses them: the window of the credentials is answered, and the error is what is said
    await waitFor(async () => (await b.$$('[role="dialog"]')).length === 1, "the window of the credentials never showed");
    await b.$("#credentials-username").setValue("carla.v");
    await b.$("#credentials-password").setValue("not-the-password");
    const mark = speechMark();
    await b.$('[role="dialog"] button[type="submit"]').click();
    await waitForSpeech(mark, /Authentication failed/, 15_000);
    await settle(1500);
    const lines = announced().filter((line) => /Authentication failed/.test(line));
    record("K5b an error of the connection, said by Rust", [...speechSince(mark), ...lines], atspiTree());
    expect(said(mark, /Connection to Ufficio Napoli failed/), "said once").toHaveLength(1);
    expect(lines.length, "Orca dequeued an object:announcement carrying the error").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain("(2, 0, Connection to Ufficio Napoli failed");
  });

  it("K5c_the_credentials_dialog_is_read_as_a_dialog", async () => {
    await openWith("credentials");
    await choose(b, "Datacenter Milano");
    const before = announced().length;
    const mark = speechMark();
    await b.$("button=Connect").click();
    const tree = await waitForTree((t) => !!t.find((n) => (n.role === "dialog" || n.role === "alert") && /^Credentials for «Datacenter Milano»$/.test(n.name)), "the window of the credentials open");
    const dialog = tree.find((n) => (n.role === "dialog" || n.role === "alert") && /^Credentials for «Datacenter Milano»$/.test(n.name)) as AtNode;
    // the focus is on the first field, inside the dialog, and nothing of the page behind is reachable
    await waitForTree((t) => focusedNode(t)?.name === "User name", "the focus on «User name»");
    expect(descendants(tree, dialog).some((n) => n.states.has("focused"))).toBe(true);
    expect(findNode(tree, "page tab", "General")).toBeUndefined();
    // Both are required: an empty field read as «blank» is not a reading of this dialog. Wait for the title AND the field,
    // not just for the focus in the tree: AT-SPI's focus can arrive before Orca speaks.
    await waitForSpeech(mark, "Credentials for «Datacenter Milano»");
    const spoken = await waitForSpeech(mark, "User name");
    expect(spoken.some((line) => line.includes("Credentials for «Datacenter Milano»"))).toBe(true);
    expect(spoken.some((line) => line.includes("User name"))).toBe(true);
    record("K5c the window of the credentials, opened and read", spoken, tree);
    // it takes the focus and does not announce: `connecting` and `needs_input` are not said (Q27, rule 2)
    await settle(1500);
    expect(announced().slice(before), "no announcement was sent for the window of the credentials").toEqual([]);
  });

  it("K5d_the_confirmation_of_a_foreign_session_is_read", async () => {
    await openWith("plain");
    const home = mkdtempSync(join(tmpdir(), "galleria-other-"));
    const file = join(home, "other.conf");
    writeFileSync(file, "[Interface]\nPrivateKey = OTHER-RUBRICA-PRIVATE\nAddress = 10.7.0.2/32\n\n[Peer]\nPublicKey = U0VFRC1QVUJMSUMtdnBzLi4uLi4uLi4uLi4uLi4uLi4=\nEndpoint = other.example:51820\nAllowedIPs = 0.0.0.0/0\n");
    const other = (...args: string[]) => execFileSync(env("GALLERIA_SMOKE_CLI"), ["--socket", env("GALLERIA_SOCKET"), ...args], { encoding: "utf8", env: { ...process.env, XDG_CONFIG_HOME: home } });
    other("import", file, "--name", "Altra rubrica");
    other("connect", "Altra rubrica");
    const group = '[role="group"][aria-label="Session started outside this rubrica"]';
    await waitFor(async () => await b.$(group).isExisting(), "the list never showed the fixed row of the session of someone else");
    const mark = speechMark();
    await b.$(`${group} button[aria-label^="Disconnect"]`).click();
    const tree = await waitForTree((t) => !!t.find((n) => (n.role === "alert" || n.role === "dialog") && /^Disconnect «Altra rubrica»\?$/.test(n.name)), "the confirmation open");
    const dialog = tree.find((n) => (n.role === "alert" || n.role === "dialog") && /^Disconnect «Altra rubrica»\?$/.test(n.name)) as AtNode;
    // the role is the one of an alert dialog, the focus is on the safe answer, and the text is its description
    expect(focusedNode(tree)?.name).toBe("Cancel");
    expect(descendants(tree, dialog).some((n) => n.states.has("focused"))).toBe(true);
    const text = "This connection was not started from this rubrica. Another user of this computer may be using it. Disconnect it?";
    expect(dialog.description.includes(text) || dialog.relations.some((r) => r.startsWith("described-by")), `the text is the description: ${dialog.description}`).toBe(true);
    const spoken = await waitForSpeech(mark, /Disconnect «Altra rubrica»\?/);
    record(`K5d the confirmation of a session of someone else (role seen: ${dialog.role})`, spoken, tree);
    expect(spoken.join(" ")).toContain("Cancel");
    // Escape is «Cancel»: the session goes on, and the focus is not lost
    await b.keys(["Escape"]);
    await waitForTree((t) => !t.find((n) => /^Disconnect «Altra rubrica»\?$/.test(n.name)) && !!focusedNode(t), "the confirmation closed and something focused");
    expect(other("status").trim()).toBe("connected");
  });

  it("K5f_the_switch_says_disconnected_then_connected", async () => {
    await openWith("plain");
    await b.$("button=Connect").click();
    await waitFor(async () => (await b.$('[data-cell="strip.state"] dd').getText()).trim() === "Connected", "the first profile never connected");
    await choose(b, "Homelab");
    const mark = speechMark();
    await b.$("button=Connect").click();
    const NOTICE = "Disconnected from «Ufficio Napoli»";
    const STATE = "Connected to Homelab";
    await waitForSpeech(mark, STATE, 15_000);
    await settle(1500);
    const all = speechSince(mark);
    record("K5f the switch of profile: the notice and the state", all, atspiTree());
    expect(said(mark, NOTICE), `the notice: ${JSON.stringify(all)}`).toHaveLength(1);
    expect(said(mark, STATE), `the state: ${JSON.stringify(all)}`).toHaveLength(1);
    // in this order: what was left, then what was reached
    expect(all.findIndex((line) => line.includes(NOTICE))).toBeLessThan(all.findIndex((line) => line.includes(STATE)));
  });
});
