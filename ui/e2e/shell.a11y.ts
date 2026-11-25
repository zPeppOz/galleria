// Plan B2 T7, the shell under a screen reader (Q22 (a): Orca and the AT-SPI tree in the real window, run locally by
// lab/tests/gui-a11y.sh, i.e. `scripts/gui.sh a11y`). One session of the app, in order: the list, the tabs, the switch,
// the confirmation of «Delete», a notice that goes away by itself. Assertions are on our own names and on the states
// AT-SPI reports; the words Orca uses for the role and the state are recorded in target/gui-a11y/transcript.txt and
// copied into docs/08-esito-fase2.md, because they are Orca's, not ours.
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  atspiTree, closeApp, daemon, descendants, findNode, focusedNode, openApp, record, speechMark, speechSince, waitForBody, waitForSpeech, waitForTree,
  type AtNode, type Browser,
} from "./support";

let b: Browser;

const key = (...keys: string[]) => b.keys(keys);
const focusedName = (tree: AtNode[]) => focusedNode(tree)?.name;
const focusedIs = (name: string | RegExp) => (tree: AtNode[]) => {
  const got = focusedName(tree) ?? "";
  return typeof name === "string" ? got === name : name.test(got);
};

/** Presses `keys` until the focus is on the node called `name`; fails after `limit` presses. */
async function reach(name: string | RegExp, keys: string[], limit = 14): Promise<AtNode[]> {
  for (let step = 0; step < limit; step++) {
    const tree = atspiTree();
    if (focusedIs(name)(tree)) return tree;
    await key(...keys);
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  return waitForTree(focusedIs(name), `${String(name)} focused after ${limit} presses of ${keys.join("+")}`, 3000);
}

beforeAll(async () => {
  daemon("start");
  b = await openApp();
  await waitForBody(b, "Connected to the service", "the shell never showed the link");
  // every securitypolicyviolation event from here on is kept and read at the end
  await b.execute(() => {
    const w = window as unknown as { __csp: string[] };
    w.__csp = [];
    document.addEventListener("securitypolicyviolation", (e) => w.__csp.push(`${e.violatedDirective} ${e.blockedURI}`));
  });
});

afterAll(closeApp);

describe("the shell under a screen reader", () => {
  it("S1_list_is_a_named_listbox_and_the_focused_row_is_spoken", async () => {
    const mark = speechMark();
    await key("Tab");
    await waitForTree(focusedIs("Menu"), "the Menu button focused");
    await key("Tab");
    const tree = await waitForTree(focusedIs("Ufficio Napoli"), "the chosen row «Ufficio Napoli» focused");
    const listbox = findNode(tree, "list box", "Connections");
    expect(listbox, "a list box named «Connections»").toBeTruthy();
    const rows = descendants(tree, listbox as AtNode).filter((n) => n.role === "list item");
    expect(rows.map((n) => n.name)).toEqual(["Ufficio Napoli", "Datacenter Milano", "Homelab", "VPS Hetzner", "Cliente Rossi"]);
    const row = focusedNode(tree) as AtNode;
    expect(row.states.has("selected")).toBe(true);
    expect(rows.filter((n) => n.states.has("selected"))).toHaveLength(1);
    // the server and the protocol are the description of the row
    expect(row.description + row.relations.join(",")).toMatch(/vpn\.ufficio\.example|OpenVPN|described-by/);
    record(`S1 list (role of a row: ${row.role}; description: «${row.description}»)`, await waitForSpeech(mark, "Ufficio Napoli"), tree);
  });

  it("S2_arrow_moves_the_focus_and_the_choice_and_the_new_row_is_spoken", async () => {
    const mark = speechMark();
    await key("ArrowDown");
    const tree = await waitForTree(focusedIs("Datacenter Milano"), "«Datacenter Milano» focused after ↓");
    expect((focusedNode(tree) as AtNode).states.has("selected")).toBe(true);
    expect(await b.$("h1").getText()).toBe("Datacenter Milano");
    record("S2 ArrowDown in the list", await waitForSpeech(mark, "Datacenter Milano"), tree);
  });

  it("S3_tabs_are_a_page_tab_list_and_the_chosen_tab_is_spoken", async () => {
    await key("Tab");
    await waitForTree(focusedIs("More actions"), "«More actions» focused");
    const mark = speechMark();
    await key("Tab");
    let tree = await waitForTree(focusedIs("General"), "the tab «General» focused");
    const strip = findNode(tree, "page tab list", "Profile sections");
    expect(strip, "a page tab list named «Profile sections»").toBeTruthy();
    const tabs = descendants(tree, strip as AtNode).filter((n) => n.role === "page tab");
    expect(tabs.map((n) => n.name)).toEqual(["General", "Authentication", "Network", "Configuration", "Log"]);
    expect(tabs.filter((n) => n.states.has("selected")).map((n) => n.name)).toEqual(["General"]);
    record("S3 the tab strip reached", await waitForSpeech(mark, "General"), tree);

    const next = speechMark();
    await key("ArrowRight");
    tree = await waitForTree(focusedIs("Authentication"), "«Authentication» focused after →");
    expect((focusedNode(tree) as AtNode).states.has("selected")).toBe(true);
    record("S3 ArrowRight on the tabs", await waitForSpeech(next, "Authentication"), tree);
    await key("ArrowLeft");
    await waitForTree(focusedIs("General"), "«General» focused again");
  });

  it("S4_switch_state_is_exposed_and_spoken", async () => {
    const reached = speechMark();
    const tree = await reach(/Reconnect automatically/, ["Tab"]);
    const control = focusedNode(tree) as AtNode;
    expect(["switch", "toggle button"]).toContain(control.role);
    expect(control.states.has("checked") || control.states.has("pressed")).toBe(true);
    record(`S4 switch reached (role seen: ${control.role})`, await waitForSpeech(reached, /Reconnect automatically/), tree);
    const mark = speechMark();
    await key(" ");
    const off = await waitForTree((t) => {
      const node = focusedNode(t);
      return !!node && /Reconnect automatically/.test(node.name) && !node.states.has("checked") && !node.states.has("pressed");
    }, "the switch off after Space");
    // Orca says the new state only (the name was said when the focus arrived)
    record(`S4 switch turned off (role seen: ${control.role})`, await waitForSpeech(mark, /off|not (pressed|checked)|unchecked/i), off);
    const again = speechMark();
    await key(" ");
    const on = await waitForTree((t) => !!focusedNode(t) && (focusedNode(t)?.states.has("checked") || focusedNode(t)?.states.has("pressed")) === true, "the switch on again");
    record("S4 switch turned on", await waitForSpeech(again, /\bon\b|pressed|checked/i), on);
  });

  it("S5_confirmation_is_an_alert_dialog_with_title_and_text_spoken", async () => {
    await reach("More actions", ["Shift", "Tab"], 20);
    await key("Enter");
    await waitForTree(focusedIs("Duplicate"), "the menu open on «Duplicate»");
    await key("End");
    await waitForTree(focusedIs("Delete"), "«Delete» focused");
    const mark = speechMark();
    await key("Enter");
    const tree = await waitForTree((t) => !!t.find((n) => (n.role === "alert" || n.role === "dialog") && /^Delete Datacenter Milano\?$/.test(n.name)), "the confirmation open");
    const dialog = tree.find((n) => (n.role === "alert" || n.role === "dialog") && /^Delete Datacenter Milano\?$/.test(n.name)) as AtNode;
    expect(focusedName(tree)).toBe("Cancel");
    expect(descendants(tree, dialog).some((n) => n.states.has("focused"))).toBe(true);
    const text = "These are sample data: nothing is deleted, whatever you choose.";
    expect(dialog.description.includes(text) || dialog.relations.some((r) => r.startsWith("described-by")), `the text is the description: ${dialog.description}`).toBe(true);
    // nothing of the page behind is reachable
    expect(findNode(tree, "page tab", "General")).toBeUndefined();
    const said = await waitForSpeech(mark, /Delete Datacenter Milano\?/);
    record(`S5 confirmation opened (role seen: ${dialog.role})`, said, tree);
    expect(said.join(" ")).toContain("Cancel");

    const closed = speechMark();
    await key("Escape");
    const after = await waitForTree((t) => !t.find((n) => /^Delete Datacenter Milano\?$/.test(n.name)) && focusedIs("More actions")(t), "the confirmation closed and «More actions» focused");
    record("S5 confirmation closed with Escape", await waitForSpeech(closed, "More actions"), after);
  });

  it("S6_notice_appears_without_moving_the_focus", async () => {
    await key("Enter");
    await waitForTree(focusedIs("Duplicate"), "the menu open on «Duplicate»");
    const mark = speechMark();
    await key("Enter");
    // the notice is in a live region that was already in the page and empty: it appears, the focus stays on the button
    const tree = await waitForTree((t) => !!findNode(t, "push button", "Dismiss the notice") && focusedIs("More actions")(t), "the notice shown and the focus on «More actions»");
    expect(findNode(tree, "push button", "Dismiss the notice")?.states.has("focused") ?? false).toBe(false);
    const region = tree.find((n) => n.role === "status bar");
    expect(region, "the live region is a status bar in the tree").toBeTruthy();
    // what Orca said in the meantime, whatever it is, goes into the transcript
    await new Promise((resolve) => setTimeout(resolve, 3000));
    record("S6 notice after «Duplicate»", speechSince(mark), tree);
  });

  // KNOWN GAP (see docs/08-esito-fase2.md, «Avviso a scomparsa»): Orca 46 with WebKitGTK 2.52.6 receives the
  // `children-changed:add` of the region (politeness `polite`, atomic) and says nothing. `it.fails` keeps the gap in view:
  // the day this starts to pass, the run says so and the gap can be closed in the outcome document.
  it.fails("S6b_notice_is_spoken_by_orca", async () => {
    await key("Enter");
    await waitForTree(focusedIs("Duplicate"), "the menu open on «Duplicate»");
    const mark = speechMark();
    await key("Enter");
    await waitForSpeech(mark, /Sample data: nothing was changed/, 4000);
  });

  it("X3_no_csp_violation_in_the_shell_tour", async () => {
    const violations = await b.execute(() => (window as unknown as { __csp: string[] }).__csp);
    expect(violations).toEqual([]);
  });
});
