// Plan B3 T5, the form of a profile under a screen reader (Q22 (a): Orca and the AT-SPI tree in the real window, run
// locally by lab/tests/gui-a11y.sh, i.e. `scripts/gui.sh a11y`). Each check opens the app on a fresh seed, so what one
// does to the profiles does not reach the next. What Orca said goes into target/gui-a11y/transcript.txt.
//   R4a  a save is said once, as a notice (Q27: an outcome that does not ask for an answer is *polite*)
//   R4d  an error is said, and the window says it as *assertive*: the profile was written by someone else meanwhile
//   R4e  an export through the real «Save» window (GTK, driven over AT-SPI) is said once, as a notice; R4f the cancel of it says nothing
//   R5a  an error of a field takes the focus, and it is read with the field (Q27, rule 2: no notice for it)
import { mkdtempSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  answerSaveWindow, atspiTree, bodyText, closeApp, daemon, env, findNode, focusedNode, openApp, orcaDebugLines, record, speechMark, speechSince, waitForBody, waitForSpeech, waitForTree,
  type AtNode, type Browser,
} from "./support";

let b: Browser;

const key = (...keys: string[]) => b.keys(keys);
const focusedIs = (name: string | RegExp) => (tree: AtNode[]) => {
  const got = focusedNode(tree)?.name ?? "";
  return typeof name === "string" ? got === name : name.test(got);
};
const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** Presses `keys` until the focus is on the node called `name`; fails after `limit` presses. */
async function reach(name: string | RegExp, keys: string[], limit = 14): Promise<AtNode[]> {
  for (let step = 0; step < limit; step++) {
    const tree = atspiTree();
    if (focusedIs(name)(tree)) return tree;
    await key(...keys);
    await settle(150);
  }
  return waitForTree(focusedIs(name), `${String(name)} focused after ${limit} presses of ${keys.join("+")}`, 3000);
}

/** The tab «Network» of the profile that is chosen: from the list, to the strip, and the arrows. */
async function openNetworkTab(): Promise<void> {
  await reach("More actions", ["Tab"], 14);
  await key("Tab");
  await waitForTree(focusedIs("General"), "the tab strip focused");
  await key("ArrowRight");
  await key("ArrowRight");
  await waitForTree(focusedIs("Network"), "the tab «Network» focused");
}

const profilesDir = () => join(env("XDG_CONFIG_HOME"), "galleria", "profiles");

beforeEach(async () => {
  daemon("start");
  b = await openApp();
  await waitForBody(b, "Connected to the service", "the shell never showed the link");
});

afterEach(closeApp);

describe("the form of a profile under a screen reader", () => {
  const SAVED = "Profile saved.";

  it("R4a_orca_says_saved_once", async () => {
    await reach("Name", ["Tab"]);
    // a field that gets the focus with Tab has its text selected: these keys replace the name
    await key(...Array.from("Sede"));
    await waitForBody(b, "You have changes that are not saved.", "the save bar never appeared");
    const mark = speechMark();
    await key("Control", "s");
    await waitForSpeech(mark, SAVED, 5000);
    // once: the notice is in the page too, and if Orca ever starts to say the live region this counts two
    await settle(1500);
    const times = speechSince(mark).filter((line) => line.includes(SAVED)).length;
    const tree = atspiTree();
    record("R4a the save of a name, Ctrl+S", speechSince(mark), tree);
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    // the window sent it as *polite*: Orca's debug file shows `(urgency, 0, text)` for an `object:announcement`, 1 is polite
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(SAVED));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${SAVED})`);
    // the focus stays where it was
    expect(focusedIs("Name")(tree), "the focus is still on the field «Name»").toBe(true);
    // what was written is the new name
    const files = readdirSync(profilesDir()).map((name) => readFileSync(join(profilesDir(), name), "utf8"));
    expect(files.some((text) => text.includes('name = "Sede"'))).toBe(true);
  });

  it("R4d_orca_says_an_error_as_assertive", async () => {
    await openNetworkTab();
    // someone else (the CLI) writes the profile now: the same file with another time of modification
    const file = readdirSync(profilesDir()).map((name) => join(profilesDir(), name)).find((path) => readFileSync(path, "utf8").includes('name = "Ufficio Napoli"'));
    expect(file, "the file of «Ufficio Napoli»").toBeTruthy();
    const text = readFileSync(file as string, "utf8");
    writeFileSync(file as string, text.replace(/modified = "[^"]*"/, 'modified = "2030-01-01T00:00:00Z"'), { mode: 0o600 });
    await reach(/Ignore the routes/, ["Tab"]);
    const mark = speechMark();
    await key(" ");
    const CHANGED = "This profile was changed somewhere else since you opened it.";
    await waitForSpeech(mark, CHANGED, 5000);
    await settle(1500);
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(CHANGED));
    record("R4d the write of a switch after the file was changed (error, assertive)", [...speechSince(mark), ...lines], atspiTree());
    expect(speechSince(mark).filter((line) => line.includes(CHANGED)), "said once").toHaveLength(1);
    // the window sent it as *assertive*: `(2, 0, text)` in Orca's debug file (the polite one of R4a is `(1, 0, text)`)
    expect(lines.length, "Orca dequeued an object:announcement carrying the error").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(2, 0, ${CHANGED})`);
    // nothing was being edited, so the form takes the version of the other writer in without asking: no «Reload» to answer
    expect(findNode(atspiTree(), "push button", "Reload"), "no question left in the page").toBeUndefined();
  });

  it("R5a_field_error_is_read_with_the_field", async () => {
    // Ufficio Napoli → Datacenter Milano → Homelab (WireGuard: it has an MTU)
    await reach("Ufficio Napoli", ["Tab"], 8);
    await key("ArrowDown");
    await key("ArrowDown");
    await waitForTree(focusedIs("Homelab"), "«Homelab» chosen");
    await openNetworkTab();
    await reach("MTU", ["Tab"]);
    await key(...Array.from("100"));
    // away from the field and then «Save»: the error must bring the focus back
    await reach("Keepalive (seconds)", ["Tab"]);
    const mark = speechMark();
    await key("Control", "s");
    const tree = await waitForTree(focusedIs("MTU"), "the focus back on the field «MTU»");
    const field = focusedNode(tree) as AtNode;
    const ERROR = "Use a number from 576 to 9000.";
    expect(field.states.has("invalid") || field.states.has("invalid-entry"), `the field is marked invalid: ${[...field.states].join(",")}`).toBe(true);
    expect(field.description.includes(ERROR) || field.relations.some((r) => r.startsWith("described-by")), `the error is the description: ${field.description}`).toBe(true);
    // KNOWN GAP OF THIS IMAGE ONLY (docs/08, «Buco 1» and «Spike Orca»): Orca 46.1 in the image says neither the name nor the
    // description of a text field when the focus comes to it. The tree above is right (invalid, described by the error), and
    // that is what a screen reader that says descriptions reads; on a real GNOME session this was not confirmed either way
    // (the spike did not try a description). The wait is turned upside down so the day the image says it, this turns positive.
    await expect(waitForSpeech(mark, /576 to 9000/, 2500), "known gap of the image (Orca 46.1): the error of a field is not said").rejects.toThrow(/Orca never said/);
    record("R5a an MTU that is not valid, Ctrl+S (Orca 46.1 in the image does not say the error)", speechSince(mark), tree);
    // no notice for it: it is read with the field, not announced besides
    expect(findNode(tree, "push button", "Dismiss the notice"), "no notice").toBeUndefined();
  });

  /** The menu «More actions» of the chosen profile, from the list: Tab to the button, Enter; the focus is on the first entry. */
  async function openActionsMenu(): Promise<void> {
    await reach("More actions", ["Tab"], 14);
    await key("Enter");
    await waitForTree(focusedIs("Duplicate"), "the menu open on «Duplicate»");
  }

  it("R4b_orca_says_a_duplicate_once", async () => {
    await openActionsMenu();
    const mark = speechMark();
    await key("Enter");
    const COPY = "Duplicated as “Ufficio Napoli (copy)”.";
    await waitForSpeech(mark, COPY, 5000);
    await settle(1500);
    const times = speechSince(mark).filter((line) => line.includes(COPY)).length;
    const tree = atspiTree();
    record("R4b the copy of a profile, «Duplicate»", speechSince(mark), tree);
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(COPY));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${COPY})`);
    // the copy is the chosen profile and the focus is on its «More actions»
    expect(focusedIs("More actions")(tree), "the focus is on «More actions» of the copy").toBe(true);
    expect(await b.$("h1").getText()).toBe("Ufficio Napoli (copy)");
  });

  it("R4c_orca_says_a_delete_once", async () => {
    await openActionsMenu();
    await key("End");
    await waitForTree(focusedIs("Delete"), "«Delete» focused");
    await key("Enter");
    await waitForTree((t) => !!t.find((n) => (n.role === "alert" || n.role === "dialog") && /^Delete Ufficio Napoli\?$/.test(n.name)) && focusedIs("Cancel")(t), "the confirmation open with the focus on «Cancel»");
    // the answer «Delete» is one Tab away from the safe one
    await key("Tab");
    await waitForTree(focusedIs("Delete"), "the answer «Delete» focused");
    const mark = speechMark();
    await key("Enter");
    const DONE = "Deleted “Ufficio Napoli”.";
    await waitForSpeech(mark, DONE, 5000);
    await settle(1500);
    const times = speechSince(mark).filter((line) => line.includes(DONE)).length;
    const tree = atspiTree();
    record("R4c the delete of a profile, confirmed", speechSince(mark), tree);
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(DONE));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${DONE})`);
    // the focus went to the list, on the profile that took its place: not to the body of the page
    expect(focusedNode(tree)?.role, "the focus is on a row of the list").toBe("list item");
    expect(findNode(tree, "list item", "Ufficio Napoli"), "the profile is gone from the list").toBeUndefined();
    expect(readdirSync(profilesDir()).map((name) => readFileSync(join(profilesDir(), name), "utf8")).some((text) => text.includes('name = "Ufficio Napoli"'))).toBe(false);
  });

  // «Export» ends in the GTK window «Save», which belongs to Rust and is not in the page: it is driven over AT-SPI
  // (lab/tests/atspi-save-window.py). The key that confirms is not awaited, because the command answers when that window closes.
  async function askExport(): Promise<void> {
    await openActionsMenu();
    await key("ArrowDown");
    await waitForTree(focusedIs("Export"), "«Export» focused");
    await key("Enter");
    await waitForTree((t) => !!t.find((n) => (n.role === "alert" || n.role === "dialog") && /^Export Ufficio Napoli\?$/.test(n.name)), "the confirmation open");
    await reach("Export", ["Tab"], 4);
    void key("Enter").catch(() => undefined);
  }

  it("R4e_export_saves_through_the_real_save_window", async () => {
    const dir = mkdtempSync(join(tmpdir(), "galleria-export-"));
    const target = join(dir, "Ufficio.ovpn");
    await askExport();
    const mark = speechMark();
    // the window proposes the name of the profile and the extension of its protocol; the full path is typed over it
    expect(await answerSaveWindow({ save: target })).toEqual({ offered: "Ufficio Napoli.ovpn", pressed: "Save", closed: true });
    const DONE = "Exported.";
    await waitForSpeech(mark, DONE, 5000);
    await settle(1500);
    const times = speechSince(mark).filter((line) => line.includes(DONE)).length;
    const tree = atspiTree();
    record("R4e the export of a profile, saved through the real window", speechSince(mark), tree);
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(DONE));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${DONE})`);
    // the file is private, has the keys (Q31) and nothing else was left next to it
    expect(statSync(target).mode & 0o777).toBe(0o600);
    const text = readFileSync(target, "utf8");
    expect(text).toContain("<tls-crypt>");
    expect(text).toContain("SEED-SECRET-");
    expect(readdirSync(dir)).toEqual(["Ufficio.ovpn"]);
    // the page never learns the path or the keys, and the focus is back on the button that asked
    const body = await bodyText(b);
    expect(body).toContain(DONE);
    expect(body).not.toContain(dir);
    expect(body).not.toContain("SEED-SECRET-");
    expect(focusedIs("More actions")(tree), "the focus is on «More actions»").toBe(true);
  });

  it("R4f_export_cancelled_says_nothing", async () => {
    const before = readdirSync(profilesDir()).sort().map((name) => readFileSync(join(profilesDir(), name), "utf8")).join("\n");
    await askExport();
    const mark = speechMark();
    // Orca's debug file holds the whole run (R4e's «Exported.» is in it): only what comes after this point counts
    const announced = orcaDebugLines(/Dequeued object:announcement/).length;
    expect(await answerSaveWindow("cancel")).toEqual({ offered: "Ufficio Napoli.ovpn", pressed: "Cancel", closed: true });
    // a cancel is not an outcome: no notice, no error, and nothing said by the page
    await settle(2000);
    const tree = atspiTree();
    record("R4f the export of a profile, cancelled in the real window", speechSince(mark), tree);
    expect(speechSince(mark).filter((line) => /Exported|could not/i.test(line)), "Orca said nothing about an export").toEqual([]);
    expect(orcaDebugLines(/Dequeued object:announcement/).slice(announced), "no announcement was sent").toEqual([]);
    const body = await bodyText(b);
    expect(body).not.toMatch(/Exported\.|could not be saved/i);
    // the rubrica is as it was (the window of Rust never told the page where it would have written, so there is no place to look in)
    expect(readdirSync(profilesDir()).sort().map((name) => readFileSync(join(profilesDir(), name), "utf8")).join("\n")).toBe(before);
    expect(focusedIs("More actions")(tree), "the focus is back on «More actions»").toBe(true);
  });

  // The full round, in the order a person would take it. What Orca says goes to the transcript; what is asserted is the
  // tree: names, roles and the states AT-SPI reports, which a screen reader reads.
  it("R5b_search_field_and_list_states", async () => {
    // S4b of the shell file, for the search field: the name is recorded, not asserted (Orca 46.1 in the image does not say it)
    const mark = speechMark();
    await key("Control", "k");
    const tree = await waitForTree((t) => focusedNode(t)?.role === "entry" && /Search connections/.test(focusedNode(t)?.name ?? ""), "the search field focused with Ctrl+K");
    record("R5b Ctrl+K: the search field (Orca 46.1 in the image does not say the name of a text field)", speechSince(mark), tree);
    expect(focusedNode(tree)?.relations.some((r) => r.startsWith("labelled-by")) || focusedNode(tree)?.name).toBeTruthy();
    // a search with one result: the count is text of the page (a status line), the list has the one row
    await key(...Array.from("milano"));
    const narrowed = await waitForTree((t) => !!findNode(t, "list item", "Datacenter Milano") && !findNode(t, "list item", "Homelab"), "the list narrowed to «Datacenter Milano»");
    record("R5b a search with one result", speechSince(mark), narrowed);
    // the text of a status line is in the text interface of the node, not in its name: the page says it
    await waitForBody(b, "1 result", "the count of the results is not in the page");
    // nothing found: the list says so in words
    await key("Control", "a");
    // no doubled letters: the driver drops a repeated key, and a single «z» would find «VPS Hetzner»
    await key(...Array.from("qwk"));
    const none = await waitForTree((t) => !findNode(t, "list item", "Datacenter Milano"), "the list empty");
    record("R5b a search with no result", speechSince(mark), none);
    await waitForBody(b, "No connection matches your search.", "the empty list does not say so in words");
    // Esc empties the field and the five rows are back; the focus is still in the field
    await key("Control", "a");
    await key("Escape");
    const back = await waitForTree((t) => !!findNode(t, "list item", "Homelab") && !!findNode(t, "list item", "Cliente Rossi"), "the five rows back");
    expect(back.filter((n) => n.role === "list item")).toHaveLength(5);
    expect(focusedNode(back)?.role).toBe("entry");
  });

  it("R5c_empty_rubrica_says_how_to_import", async () => {
    await closeApp();
    b = await openApp(false, "empty");
    await waitForBody(b, "No connections yet", "the empty rubrica did not say so");
    const tree = await waitForTree((t) => t.some((n) => /No connections yet/.test(n.name)), "the empty text in the tree");
    expect(tree.filter((n) => n.role === "list item")).toHaveLength(0);
    record("R5c the empty rubrica", [], tree);
    // the heading is a node with its name; the paragraph that follows has its text in the text interface of the node
    expect(tree.some((n) => n.role === "heading" && n.name === "No connections yet"), "the heading is in the tree").toBe(true);
    expect(await bodyText(b), "the command to import is text of the page").toContain("galleria import");
  });

  /** The tabs of the profile: from the list to the strip, then the arrows to the tab. */
  async function openTab(name: string, presses: number): Promise<void> {
    await reach("More actions", ["Tab"], 14);
    await key("Tab");
    await waitForTree(focusedIs("General"), "the tab strip focused");
    for (let step = 0; step < presses; step++) await key("ArrowRight");
    await waitForTree(focusedIs(name), `the tab «${name}» focused`);
  }

  it("R5d_the_new_tabs_are_read_with_their_fields", async () => {
    // Authentication: the fields of the OpenVPN profile are read, not edited (D26), and the switch is a switch
    await openTab("Authentication", 1);
    const mark = speechMark();
    const auth = atspiTree();
    expect(auth.find((n) => n.role === "page tab" && n.name === "Authentication")?.states.has("selected")).toBe(true);
    record("R5d the tab «Authentication»", speechSince(mark), auth);
    // the tab «Network»: the switch of the pushed routes is a switch with a state
    await key("ArrowRight");
    const network = await waitForTree(focusedIs("Network"), "«Network» focused");
    expect(network.some((n) => ["switch", "toggle button"].includes(n.role) && /Ignore the routes/.test(n.name)), "a switch for the pushed routes").toBe(true);
    record("R5d the tab «Network»", speechSince(mark), network);
    // the raw editor: a multi-line entry named «Configuration», with the problems under it
    await key("ArrowRight");
    await waitForTree(focusedIs("Configuration"), "«Configuration» focused");
    // the panel of a tab may be a stop of its own: Tab until the editor has the focus
    let editor = atspiTree();
    for (let step = 0; step < 4 && !(focusedNode(editor)?.role === "entry" && focusedNode(editor)?.name === "Configuration"); step++) {
      await key("Tab");
      await settle(200);
      editor = atspiTree();
    }
    editor = await waitForTree((t) => focusedNode(t)?.role === "entry" && focusedNode(t)?.name === "Configuration", `the editor focused (the focus is on ${JSON.stringify(focusedNode(editor)?.role)} «${focusedNode(editor)?.name}»)`);
    const field = focusedNode(editor) as AtNode;
    expect(field.states.has("multi-line"), `a multi-line entry: ${[...field.states].join(",")}`).toBe(true);
    record("R5d the editor of the raw text (entry, multi-line)", speechSince(mark), editor);
    // a directive that runs a script: the problem is in the list as a button that takes the caret to its line
    await key("Control", "End");
    await key("Enter", ...Array.from("up /tmp/x.sh"));
    const problems = await waitForTree((t) => t.some((n) => n.role === "push button" && /· up ·/.test(n.name)), "the problem listed", 8000);
    const problem = problems.find((n) => n.role === "push button" && /· up ·/.test(n.name)) as AtNode;
    record(`R5d the list of the problems (a button: ${problem.name})`, speechSince(mark), problems);
    expect(problems.some((n) => /Problems in the configuration/.test(n.name)), "the list has a name").toBe(true);
    // Ctrl+S with a problem: nothing is written and the focus goes to the first problem
    const before = readdirSync(profilesDir()).sort().map((name) => readFileSync(join(profilesDir(), name), "utf8")).join("\n");
    await key("Control", "s");
    const refused = await waitForTree((t) => /· up ·/.test(focusedNode(t)?.name ?? ""), "the focus on the first problem");
    record("R5d the save refused: the focus on the first problem", speechSince(mark), refused);
    expect(readdirSync(profilesDir()).sort().map((name) => readFileSync(join(profilesDir(), name), "utf8")).join("\n")).toBe(before);
  });

  it("R5e_manage_groups_dialog_is_read", async () => {
    await reach("Manage groups", ["Tab"], 12);
    const mark = speechMark();
    await key("Enter");
    const tree = await waitForTree((t) => t.some((n) => n.role === "dialog" && n.name === "Manage groups"), "the dialog open");
    record("R5e «Manage groups» opened", speechSince(mark), tree);
    // the rows are entries, with a button for each move and for the delete; the new group has its own field
    for (const group of ["Lavoro", "Personale", "Clienti"]) {
      expect(tree.some((n) => n.role === "push button" && n.name === `Delete ${group}`), `a button «Delete ${group}»`).toBe(true);
      expect(tree.some((n) => n.role === "push button" && n.name === `Move ${group} down`), `a button «Move ${group} down»`).toBe(true);
    }
    expect(tree.some((n) => n.role === "entry" && n.name === "New group"), "the field of the new group").toBe(true);
    // a group is added with Enter: the notice is said (it is polite) and the new row takes the focus
    await waitForTree((t) => focusedNode(t)?.role === "entry" || focusedNode(t)?.role === "push button", "something of the dialog has the focus");
    await reach("New group", ["Tab"], 20);
    const added = speechMark();
    await key(...Array.from("Casa"), "Enter");
    await waitForSpeech(added, "Group Casa created.", 5000);
    const after = atspiTree();
    record("R5e a group added", speechSince(added), after);
    expect(after.some((n) => n.role === "entry" && n.name === "Group 4"), "the row of the new group").toBe(true);
    // Escape closes the dialog and the focus is back on its button
    await key("Escape");
    await waitForTree((t) => !t.some((n) => n.role === "dialog") && focusedIs("Manage groups")(t), "the dialog closed and the focus back on «Manage groups»");
  });
});
