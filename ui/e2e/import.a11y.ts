// Plan B4 T3, the window «Open» of the import under a screen reader (Q22 (a): Orca and the AT-SPI tree in the real window, run
// locally by lab/tests/gui-a11y.sh, i.e. `scripts/gui.sh a11y`). «Choose a file…» ends in the GTK window «Open», which belongs
// to Rust and is not in the page: it is driven over AT-SPI (lab/tests/atspi-file-window.py), like «Save» of «Export».
//   I5a  the window is a file chooser with its two buttons and its two filters
//   I5b  a file chosen in the real window is previewed and imported: one notice, polite, nothing of the folder in the page
//   I5c  «Cancel» in the real window changes nothing: no notice, no preview, the focus on the button
//   I5d  the preview is read as a dialog: its name, and the focus on «Name»
//   I5e  a file dragged onto the window with a real drag (T4) is previewed with the text of D19, and imported: one notice
//   I5f  an error of the import takes the focus (the notice has the focus, the alert is inside it) and is not announced (T7)
//   I5g  the text of D19 is in the tree, as the name of the items of the list of what the import changed (T7)
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  answerFileWindow, atspiTree, bodyText, closeApp, daemon, descendants, dragFile, dumpFileWindow, env, findNode, focusedNode, importFixtures, openApp, orcaDebugLines, record, speechMark, speechSince,
  waitForBody, waitForSpeech, waitForTree,
  type AtNode, type Browser,
} from "./support";

let b: Browser;

const key = (...keys: string[]) => b.keys(keys);
const focusedIs = (name: string | RegExp) => (tree: AtNode[]) => {
  const got = focusedNode(tree)?.name ?? "";
  return typeof name === "string" ? got === name : name.test(got);
};
const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const configDir = () => join(env("XDG_CONFIG_HOME"), "galleria");

/** Presses `keys` until the focus is on the node called `name`; fails after `limit` presses. */
async function reach(name: string | RegExp, keys: string[], limit = 8): Promise<AtNode[]> {
  for (let step = 0; step < limit; step++) {
    const tree = atspiTree();
    if (focusedIs(name)(tree)) return tree;
    await key(...keys);
    await settle(150);
  }
  return waitForTree(focusedIs(name), `${String(name)} focused after ${limit} presses of ${keys.join("+")}`, 3000);
}

/** Every file under the folder of the profiles, with its bytes: what must stay as it is until «Import». */
function onDisk(): string {
  const walk = (dir: string): string[] =>
    readdirSync(dir, { withFileTypes: true })
      .sort((x, y) => x.name.localeCompare(y.name))
      .flatMap((entry) => (entry.isDirectory() ? walk(join(dir, entry.name)) : [`${join(dir, entry.name)}\n${readFileSync(join(dir, entry.name), "utf8")}`]));
  return walk(configDir()).join("\n---\n");
}

/** «Import…» with the keyboard, then the button «Choose a file…» (the first stop of the window is the text box, below it). */
async function openTheWindowOfTheImport(): Promise<AtNode[]> {
  await key("Tab");
  await waitForTree(focusedIs("Import…"), "the Import button focused");
  await key("Enter");
  await waitForTree((t) => !!findNode(t, "dialog", "Import profile"), "the window «Import profile» open");
  return reach("Choose a file…", ["Shift", "Tab"], 6);
}

/** The key that asks is not awaited: the command answers when the window of Rust closes. */
async function askForTheFile(): Promise<void> {
  void key("Enter").catch(() => undefined);
}

beforeEach(async () => {
  daemon("start");
  b = await openApp();
  await waitForBody(b, "Connected to the service", "the shell never showed the link");
});

afterEach(closeApp);

describe("the window «Open» of the import under a screen reader", () => {
  it("I5a_the_open_window_is_a_file_chooser_with_the_two_filters", async () => {
    await openTheWindowOfTheImport();
    await askForTheFile();
    const shown = await dumpFileWindow();
    record("I5a the window «Open», dumped", [JSON.stringify(shown)], atspiTree());
    // «Open» and «Cancel» are GTK's own words, in the language of the system
    expect(shown.buttons).toEqual(expect.arrayContaining(["Open", "Cancel"]));
    expect(shown.buttons, "it is not a window «Save»").not.toContain("Save");
    // the two filters have no translated name: they are the patterns themselves (D22)
    expect(shown.filters).toEqual(expect.arrayContaining(["*.ovpn, *.conf"]));
    await answerFileWindow("cancel");
  });

  it("I5b_a_file_picked_in_the_real_window_is_previewed_and_imported", async () => {
    const files = importFixtures();
    const folder = files.separate.slice(0, files.separate.lastIndexOf("/"));
    const secrets = () => readdirSync(join(configDir(), "secrets")).length;
    const secretsBefore = secrets();
    const profilesBefore = readdirSync(join(configDir(), "profiles")).length;
    await openTheWindowOfTheImport();
    await askForTheFile();
    const answered = await answerFileWindow({ open: files.separate });
    expect(answered, JSON.stringify(answered)).toMatchObject({ pressed: "Open", closed: true });
    // the preview is the second step: the file name, the secrets by name, and nothing written yet
    await waitForBody(b, "File: separate.ovpn", "the preview of the file never showed");
    const preview = await bodyText(b);
    expect(preview).toContain("Found: private key, TLS encryption key. They will be saved in the secrets archive.");
    expect(preview, "the folder of the file is not in the page").not.toContain(folder);
    expect(preview, "the secrets are named and never shown").not.toContain("SEED-SECRET-");
    expect(readdirSync(join(configDir(), "profiles")).length, "nothing is written before «Import»").toBe(profilesBefore);
    await waitForTree(focusedIs("Name"), "the field «Name» focused");
    const mark = speechMark();
    await key("Enter");
    const DONE = "Imported “separate”.";
    await waitForSpeech(mark, DONE, 5000);
    await settle(1500);
    const times = speechSince(mark).filter((line) => line.includes(DONE)).length;
    const tree = atspiTree();
    record("I5b the import of a file chosen in the real window", speechSince(mark), tree);
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    // the window sent it as *polite*: `(urgency, 0, text)` for an `object:announcement`, 1 is polite
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(DONE));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${DONE})`);
    // the files are on the disk: private, with the keys in the archive of the secrets
    const profiles = readdirSync(join(configDir(), "profiles"));
    expect(profiles).toHaveLength(profilesBefore + 1);
    const saved = profiles.map((name) => join(configDir(), "profiles", name)).find((path) => readFileSync(path, "utf8").includes('name = "separate"'));
    expect(saved, "the profile «separate» is on the disk").toBeTruthy();
    expect(statSync(saved as string).mode & 0o777).toBe(0o600);
    expect(secrets()).toBe(secretsBefore + 1);
    // the page never learned the folder, and the new profile is the one chosen, with the focus on its row
    expect(await bodyText(b)).not.toContain(folder);
    expect(findNode(tree, "list item", "separate"), "the profile is in the list").toBeTruthy();
    expect(focusedNode(tree)?.name, "the focus is on the row of the new profile").toBe("separate");
  });

  it("I5c_cancel_in_the_real_window_changes_nothing", async () => {
    const before = onDisk();
    await openTheWindowOfTheImport();
    const mark = speechMark();
    const announced = orcaDebugLines(/Dequeued object:announcement/).length;
    await askForTheFile();
    expect(await answerFileWindow("cancel")).toMatchObject({ pressed: "Cancel", closed: true });
    // a cancel is not an outcome: no notice, no error, no preview
    await settle(2000);
    const tree = atspiTree();
    record("I5c the choice of a file, cancelled in the real window", speechSince(mark), tree);
    expect(orcaDebugLines(/Dequeued object:announcement/).slice(announced), "no announcement was sent").toEqual([]);
    const body = await bodyText(b);
    expect(body).not.toMatch(/Imported “|could not|not imported/i);
    expect(body).not.toContain("File: ");
    expect(findNode(tree, "dialog", "Import profile"), "the window of the import is still the first step").toBeTruthy();
    expect(tree.some((n) => n.role === "alert"), "no alert").toBe(false);
    expect(onDisk()).toBe(before);
    expect(focusedIs("Choose a file…")(tree), "the focus is back on the button that asked").toBe(true);
    expect(existsSync(join(configDir(), "profiles"))).toBe(true);
  });

  it("I5d_the_preview_is_read_as_a_dialog", async () => {
    const files = importFixtures();
    const mark = speechMark();
    await openTheWindowOfTheImport();
    await askForTheFile();
    await answerFileWindow({ open: files.separate });
    await waitForBody(b, "File: separate.ovpn", "the preview of the file never showed");
    const tree = await waitForTree(focusedIs("Name"), "the field «Name» focused");
    // the words Orca uses for the fields are recorded, not asserted (S4b: Orca 46.1 in the image does not say the name of a text field)
    record("I5d the preview of a file, as a dialog", speechSince(mark), tree);
    expect(findNode(tree, "dialog", "Import profile"), "a dialog named «Import profile»").toBeTruthy();
    expect(speechSince(mark).some((line) => line.includes("Import profile")), `Orca said the name of the window: ${JSON.stringify(speechSince(mark))}`).toBe(true);
    // the other fields of the step are there, named
    expect(findNode(tree, "combo box", "Group") ?? findNode(tree, "menu", "Group") ?? findNode(tree, "list box", "Group"), "the field «Group»").toBeTruthy();
    expect(tree.some((n) => n.role === "push button" && n.name === "Import")).toBe(true);
    expect(tree.some((n) => n.role === "push button" && n.name === "Back")).toBe(true);
  });
});

describe("a file dragged onto the window under a screen reader", () => {
  it("I5e_a_file_dragged_onto_the_window_is_previewed", async () => {
    const files = importFixtures();
    const folder = files.office.slice(0, files.office.lastIndexOf("/"));
    const profilesBefore = readdirSync(join(configDir(), "profiles")).length;
    const before = onDisk();
    const mark = speechMark();
    // a real drag between two windows: the file is offered as `text/uri-list` by a window of GTK, and let go over Galleria
    const dragged = await dragFile(files.office);
    record("I5e the drag", [JSON.stringify(dragged)]);
    expect(dragged, JSON.stringify(dragged)).toMatchObject({ ended: true, dropped: true });
    // the preview opens by itself, at the second step: WireGuard, the file, and the hook that was removed with the text of D19
    await waitForBody(b, "File: office.conf", "the preview of the dropped file never showed");
    const preview = await bodyText(b);
    expect(preview, "the protocol tag says WireGuard").toContain("WG");
    expect(preview).toContain("PostUp");
    expect(preview).toContain("Galleria does not run the scripts of profiles.");
    expect(preview, "the folder of the file is not in the page").not.toContain(folder);
    expect(preview, "the secrets are named and never shown").not.toContain("SEED-SECRET-");
    expect(onDisk(), "nothing is written before «Import»").toBe(before);
    const tree = await waitForTree(focusedIs("Name"), "the field «Name» focused");
    expect(findNode(tree, "dialog", "Import profile"), "a dialog named «Import profile»").toBeTruthy();
    // «Import»: one notice, with the number of directives removed, said once
    const markImport = speechMark();
    await key("Enter");
    const DONE = "Imported “office”. 1 directive removed.";
    await waitForSpeech(markImport, DONE, 5000);
    await settle(1500);
    const times = speechSince(markImport).filter((line) => line.includes(DONE)).length;
    record("I5e the import of a dropped file", speechSince(mark), atspiTree());
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(markImport))}`).toBe(1);
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(DONE));
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toContain(`(1, 0, ${DONE})`);
    expect(readdirSync(join(configDir(), "profiles"))).toHaveLength(profilesBefore + 1);
    expect(await bodyText(b)).not.toContain(folder);
  });
});

describe("the errors and the text of D19 of the import under a screen reader (T7)", () => {
  /** Opens the window of the import with the keyboard and puts `text` in the field of the first step, as a paste does. */
  async function pasteText(text: string): Promise<void> {
    await key("Tab");
    await waitForTree(focusedIs("Import…"), "the Import button focused");
    await key("Enter");
    await waitForTree((t) => !!findNode(t, "dialog", "Import profile"), "the window «Import profile» open");
    await b.execute(
      (selector: string, value: string) => {
        const el = document.querySelector(selector) as HTMLTextAreaElement;
        Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(el, value);
        el.dispatchEvent(new Event("input", { bubbles: true }));
      },
      'textarea[name="import-text"]',
      text,
    );
  }

  it("I5f_an_error_of_the_import_takes_the_focus_and_is_not_announced", async () => {
    const before = onDisk();
    const announced = orcaDebugLines(/Dequeued object:announcement/).length;
    await pasteText("client\n<key>\nnever closed\n");
    const mark = speechMark();
    await b.$("button=Preview").click();
    await waitForBody(b, "The profile was not imported", "the error of the import never showed");
    // the notice of the error is the node that has the focus, and the alert (WebKitGTK exports `role=alert` as `notification`) is inside it: Q27, it is read where the focus is
    const holdsTheAlert = (t: AtNode[]) => {
      const focused = focusedNode(t);
      return !!focused && descendants(t, focused).some((n) => n.role === "notification") && descendants(t, focused).some((n) => n.role === "image" && n.name === "Error");
    };
    const tree = await waitForTree(holdsTheAlert, "the focus on the notice of the error, with the alert inside");
    await settle(1500);
    const focus = await b.execute(() => ({ inside: Boolean(document.querySelector(".import-error")?.contains(document.activeElement)), text: document.querySelector(".import-error")?.textContent ?? "" }));
    record("I5f an error of the import", speechSince(mark), tree);
    expect(focus.inside, "the notice of the error has the focus").toBe(true);
    expect(focus.text).toContain("The profile cannot be read: there is an error at line 2.");
    expect(holdsTheAlert(atspiTree()), "after a moment the focus is still there").toBe(true);
    expect(findNode(tree, "dialog", "Import profile"), "the window is still the first step").toBeTruthy();
    // no announcement was sent for it, and nothing was written
    expect(orcaDebugLines(/Dequeued object:announcement/).slice(announced), "no announcement was sent").toEqual([]);
    expect(onDisk()).toBe(before);
  });

  it("I5g_the_text_of_d19_is_in_the_tree_as_the_text_of_the_list", async () => {
    const files = importFixtures();
    const mark = speechMark();
    await pasteText(readFileSync(files.malicious, "utf8"));
    await b.$("button=Preview").click();
    await b.$('input[name="import-name"]').waitForDisplayed({ timeout: 10_000 });
    const D19 = "Galleria does not run the scripts of profiles. If the profile depends on it, it may not work fully.";
    // WebKitGTK gives each item of the list the whole sentence as its name: the line, the directive and the reason
    const tree = await waitForTree((t) => t.some((n) => n.role === "list item" && n.name.includes(D19)), "a list item with the text of D19");
    const items = tree.filter((n) => n.role === "list item" && n.name.includes("Directive"));
    record("I5g the notices of the preview of the malicious profile", speechSince(mark), tree);
    const names = JSON.stringify(items.map((n) => n.name));
    expect(items.find((n) => n.name.includes(" up "))?.name, names).toContain(D19);
    expect(items.find((n) => n.name.includes(" plugin "))?.name, names).toContain(D19);
    // one item for each line that was taken out, in a list of its own with a title
    expect(items, names).toHaveLength(5);
    expect(tree.some((n) => n.role === "heading" && /what the import changed/i.test(n.name)), "the list has its title").toBe(true);
    // what is read is the line, the name of the directive and the reason, never an argument of it
    for (const item of items) expect(item.name).not.toMatch(/PWNED|touch|EVIL/);
  });
});
