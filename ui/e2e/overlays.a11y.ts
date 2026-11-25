// Q12, the acceptance table of the plan B2 T1 (columns b and c): the menu and the modal dialog, in the real window,
// seen through AT-SPI (b) and heard through Orca (c). Run by lab/tests/gui-a11y.sh (scripts/gui.sh a11y).
//
// The rows share one session of the app, in order: the menu button, the menu, a choice, the dialog and back.
// Assertions are on our own names (title, entries) and on the states AT-SPI reports; what Orca says for the
// role and the state is recorded in target/gui-a11y/transcript.txt and copied into docs/08-esito-fase2.md.
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  atspiTree, closeApp, daemon, descendants, findNode, focusedNode, openApp, record, speechMark, waitForBody, waitForSpeech, waitForTree,
  type AtNode, type Browser,
} from "./support";

let b: Browser;

const key = (...keys: string[]) => b.keys(keys);
const focusedName = (tree: AtNode[]) => focusedNode(tree)?.name;

beforeAll(async () => {
  daemon("stop");
  b = await openApp();
  await waitForBody(b, "Start the service", "the window never rendered");
  // X2: every securitypolicyviolation event from here on is kept, to be read after the rows below.
  await b.execute(() => {
    const w = window as unknown as { __csp: string[] };
    w.__csp = [];
    document.addEventListener("securitypolicyviolation", (e) => w.__csp.push(`${e.violatedDirective} ${e.blockedURI}`));
  });
});

afterAll(closeApp);

describe("menu and dialog under a screen reader", () => {
  it("control_how_webkitgtk_exposes_a_plain_popup_button", async () => {
    // A bare <button aria-haspopup="menu" aria-expanded="true"> with no library: what AT-SPI shows for it is what
    // WebKitGTK does, not what Base UI did. The result is recorded and compared with the real trigger below.
    await b.execute(() => {
      const button = document.createElement("button");
      button.id = "control-popup";
      button.textContent = "Control";
      button.setAttribute("aria-haspopup", "menu");
      button.setAttribute("aria-expanded", "true");
      document.body.append(button);
    });
    const tree = await waitForTree((t) => !!t.find((n) => n.name === "Control"), "the control button in the tree");
    const control = tree.find((n) => n.name === "Control") as AtNode;
    record(`control: plain button with aria-haspopup=menu and aria-expanded=true (role seen: ${control.role}, expanded in AT-SPI: ${control.states.has("expanded")})`, [], [control]);
    await b.execute(() => document.getElementById("control-popup")?.remove());
  });

  it("M1_trigger_exposes_haspopup_and_expanded", async () => {
    const mark = speechMark();
    await key("Tab");
    const tree = await waitForTree((t) => focusedName(t) === "Menu", "the Menu button focused");
    const trigger = focusedNode(tree);
    expect(trigger?.states.has("has-popup")).toBe(true);
    // WebKitGTK maps a button with aria-haspopup to a `combo box` and does not export `expanded` for it (see the
    // control above), so the state itself is read from the DOM, where the library sets it.
    expect(await b.$("button=Menu").getAttribute("aria-expanded")).toBe("false");
    record(`M1 trigger (role seen: ${trigger?.role})`, await waitForSpeech(mark, "Menu"), tree);
  });

  it("M2_menu_opens_from_the_keyboard_on_the_first_item", async () => {
    const mark = speechMark();
    await key("Enter");
    const tree = await waitForTree((t) => focusedName(t) === "System", "the first entry «System» focused");
    expect(findNode(tree, "menu", "Menu")).toBeTruthy();
    expect(await b.$("button=Menu").getAttribute("aria-expanded")).toBe("true");
    // The tree ties the button to its menu even where it does not export `expanded`.
    expect(findNode(tree, "menu", "Menu")?.relations.some((r) => r.startsWith("controlled-by"))).toBe(true);
    record("M2 menu opened", await waitForSpeech(mark, "System"), tree);
  });

  it("M3_arrow_home_end_and_typeahead_move_the_focus", async () => {
    for (const [press, expected] of [["ArrowDown", "English"], ["End", "About Galleria"], ["Home", "System"], ["a", "About Galleria"]] as const) {
      const mark = speechMark();
      await key(press);
      const tree = await waitForTree((t) => focusedName(t) === expected, `${expected} focused after ${press}`);
      record(`M3 ${press}`, await waitForSpeech(mark, expected), tree);
    }
  });

  it("M4_radio_items_expose_and_change_their_checked_state", async () => {
    await key("Home");
    let tree = await waitForTree((t) => focusedName(t) === "System", "System focused");
    const radios = tree.filter((n) => n.role === "radio menu item");
    // Two groups since T3: the language, then the theme.
    expect(radios.map((n) => n.name)).toEqual(["System", "English", "Italiano", "System", "Light", "Dark"]);
    expect(radios.filter((n) => n.states.has("checked")).map((n) => n.name)).toEqual(["System", "System"]);
    await key("ArrowDown");
    await waitForTree((t) => focusedName(t) === "English", "English focused");
    const mark = speechMark();
    await key("Enter");
    // M5 (a choice closes the menu): the entries are gone and the focus is back on the button.
    tree = await waitForTree((t) => !findNode(t, "menu", "Menu") && focusedName(t) === "Menu", "the menu closed and Menu focused");
    record("M4/M5 English chosen", await waitForSpeech(mark, "Menu"), tree);
    const reopened = speechMark();
    await key("Enter");
    tree = await waitForTree((t) => focusedName(t) === "System", "the menu reopened on its first entry");
    expect(tree.filter((n) => n.role === "radio menu item" && n.states.has("checked")).map((n) => n.name)).toEqual(["English", "System"]);
    record("M4 reopened, checked entry", await waitForSpeech(reopened, "System"), tree);
  });

  it("M5_escape_closes_the_menu_and_returns_focus", async () => {
    await key("Escape");
    const tree = await waitForTree((t) => !findNode(t, "menu", "Menu") && focusedName(t) === "Menu", "the menu closed and Menu focused");
    const trigger = focusedNode(tree);
    expect(trigger?.states.has("expanded")).toBe(false);
  });

  it("D1_dialog_opens_with_focus_inside_and_is_named_by_its_title", async () => {
    // The menu is open (reopened by M4) with the focus on its first entry; Escape of M5 closed it, so open it again.
    await key("Enter");
    await waitForTree((t) => focusedName(t) === "System", "menu open on its first entry");
    await key("End");
    await waitForTree((t) => focusedName(t) === "About Galleria", "About Galleria focused");
    const mark = speechMark();
    await key("Enter");
    const tree = await waitForTree((t) => !!findNode(t, "dialog", "About Galleria") && !!focusedNode(t), "the dialog «About Galleria» open with a focused node");
    const dialog = findNode(tree, "dialog", "About Galleria") as AtNode;
    const inside = descendants(tree, dialog);
    const focused = inside.find((n) => n.states.has("focused"));
    expect(focused, "a descendant of the dialog has the focus").toBeTruthy();
    record("D1 dialog opened", await waitForSpeech(mark, "About Galleria"), tree);
  });

  it("D2_tab_stays_inside_and_the_page_behind_is_unreachable", async () => {
    for (const keys of [["Tab"], ["Tab"], ["Tab"], ["Tab"], ["Shift", "Tab"], ["Shift", "Tab"]]) {
      await key(...keys);
      const tree = await waitForTree((t) => !!focusedNode(t), "something focused");
      const dialog = findNode(tree, "dialog", "About Galleria") as AtNode;
      const inside = descendants(tree, dialog);
      expect(inside.some((n) => n.states.has("focused")), `focus inside the dialog after ${keys.join("+")}`).toBe(true);
    }
    const tree = atspiTree();
    // What the first-run panel shows behind the dialog (its buttons) is not in the tree while the dialog is open.
    expect(findNode(tree, "push button", "Try again")).toBeUndefined();
    expect(findNode(tree, "push button", "Copy")).toBeUndefined();
    record("D2 tab cycle done", [], tree);
  });

  it("D3_dialog_is_described_by_its_description", async () => {
    const tree = atspiTree();
    const dialog = findNode(tree, "dialog", "About Galleria") as AtNode;
    const described = dialog.relations.some((r) => r.startsWith("described-by")) || dialog.description.includes("A VPN client for WireGuard and OpenVPN.");
    expect(described, `description or relation on the dialog: ${formatDialog(dialog)}`).toBe(true);
    record("D3 description", [], [dialog]);
  });

  it("D4_escape_closes_and_returns_focus_to_the_opener", async () => {
    const mark = speechMark();
    await key("Escape");
    const tree = await waitForTree((t) => !findNode(t, "dialog", "About Galleria") && focusedName(t) === "Menu", "the dialog closed and Menu focused");
    record("D4 closed", await waitForSpeech(mark, "Menu"), tree);
  });

  it("X2_no_csp_violation_while_using_dialog_and_menu", async () => {
    const violations = await b.execute(() => (window as unknown as { __csp: string[] }).__csp);
    expect(violations).toEqual([]);
  });
});

const formatDialog = (n: AtNode) => `${n.role} «${n.name}» description=«${n.description}» relations=${n.relations.join(",")}`;
