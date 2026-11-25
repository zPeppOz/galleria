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

  // KNOWN GAP (docs/08-esito-fase2.md, «Campi di testo»): the tree is right (entry «Name», labelled-by) and Orca 46 with
  // WebKitGTK 2.52.6 says the value and «selected», not the name. Only the wait for the speech is turned upside down:
  // the focus and the tree are asserted as usual, so a failure to get there is a failure, not the gap. The day Orca
  // says «Name» the last line fails, and the gap can be closed in the outcome document.
  it("S4b_text_field_name_is_spoken_by_orca", async () => {
    const mark = speechMark();
    const tree = await reach("Name", ["Tab"]);
    const field = focusedNode(tree) as AtNode;
    expect(field.role).toBe("entry");
    expect(field.relations.some((r) => r.startsWith("labelled-by"))).toBe(true);
    await expect(waitForSpeech(mark, /\bName\b/, 2500), "known gap: Orca does not say the name of a text field").rejects.toThrow(/Orca never said/);
    record("S4b text field reached (Orca does not say the name)", speechSince(mark), tree);
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
  // `children-changed:add` of the region and says nothing, whatever the form of the live region (the control below).
  // Only the wait for the speech is turned upside down: opening the menu and showing the notice are asserted as usual,
  // so a failure of either is a failure, not the gap. The day Orca says it, the last line fails.
  it("S6b_notice_is_spoken_by_orca", async () => {
    await key("Enter");
    await waitForTree(focusedIs("Duplicate"), "the menu open on «Duplicate»");
    const mark = speechMark();
    await key("Enter");
    await waitForTree((t) => !!findNode(t, "push button", "Dismiss the notice") && focusedIs("More actions")(t), "the notice shown and the focus on «More actions»");
    await expect(waitForSpeech(mark, /Sample data: nothing was changed/, 4000), "known gap: Orca does not say a live region").rejects.toThrow(/Orca never said/);
  });

  // Controls for the two gaps, in the style of `control_how_webkitgtk_exposes_a_plain_popup_button`: bare elements put
  // in the page, nothing of ours. They assert what Orca does today, so the day it changes the run says so.
  const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  async function said(act: () => Promise<unknown>, ms: number): Promise<string> {
    const mark = speechMark();
    await act();
    await settle(ms);
    return speechSince(mark).join(" | ");
  }

  it("control_orca_does_not_say_the_name_of_a_bare_text_field", async () => {
    // One field at a time, reached with Tab from a bare button, in nine forms. The tree has the right name for every
    // one of them. Without anything of ours Orca says the value and «selected» (or «Text unselected.»), not the name.
    const forms = ["aria-label empty", "aria-label", "label for", "label wrapping", "aria-labelledby", "textarea", "readonly", "password", "aria-describedby"] as const;
    const words = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india"];
    for (const [n, form] of forms.entries()) {
      const name = `Probe ${words[n]}`;
      await b.execute(
        (form: string, name: string) => {
          const box = document.createElement("div");
          box.id = "control-field";
          const before = document.createElement("button");
          before.id = "control-before";
          before.textContent = "Before the field";
          box.append(before);
          const input = document.createElement(form === "textarea" ? "textarea" : "input");
          input.id = "control-input";
          input.value = form === "aria-label empty" ? "" : "Some value";
          if (form === "password") (input as HTMLInputElement).type = "password";
          if (form === "readonly") input.readOnly = true;
          if (["aria-label empty", "aria-label", "textarea", "readonly", "password"].includes(form)) input.setAttribute("aria-label", name);
          if (form === "label for" || form === "aria-describedby") {
            const label = document.createElement("label");
            label.htmlFor = input.id;
            label.textContent = name;
            box.append(label);
          }
          if (form === "aria-describedby") {
            const hint = document.createElement("p");
            hint.id = "control-hint";
            hint.textContent = "A hint";
            box.append(hint);
            input.setAttribute("aria-describedby", hint.id);
          }
          if (form === "aria-labelledby") {
            const label = document.createElement("span");
            label.id = "control-label";
            label.textContent = name;
            box.append(label);
            input.setAttribute("aria-labelledby", label.id);
          }
          if (form === "label wrapping") {
            const label = document.createElement("label");
            label.append(name, input);
            box.append(label);
          } else box.append(input);
          document.body.append(box);
          before.focus();
        },
        form,
        name,
      );
      await waitForTree(focusedIs("Before the field"), "the bare button focused");
      await settle(800);
      const mark = speechMark();
      await key("Tab");
      const tree = await waitForTree((t) => focusedName(t)?.includes(name) ?? false, `the bare field «${name}» focused (${form})`);
      await settle(2500);
      const heard = speechSince(mark);
      await b.execute(() => document.getElementById("control-field")?.remove());
      const names = heard.join(" ").includes(name);
      record(`control: bare text field, ${form} (Orca ${names ? "says" : "does not say"} the name)`, heard, [focusedNode(tree) as AtNode]);
      // The empty field is recorded and not asserted: Orca says its name on arrival in some runs («Probe alpha entry.»,
      // «Probe alpha empty entry.» in the review) and nothing at all in others. The eight fields with a value are the
      // gap: the name is never said (the textarea says nothing at all).
      if (form !== "aria-label empty") expect(names, `${form}: Orca said ${JSON.stringify(heard)}`).toBe(false);
    }
  });

  it("control_orca_does_not_say_any_form_of_live_region", async () => {
    // Ten forms put in the page, none of ours. `children-changed:add` reaches Orca and its WebKitGtk script does not
    // present it, `role=alert` included: the confirmation is heard because the focus goes into it.
    const forms = [
      "alert present, textContent", "alert new with text", "alert new with text and button", "alert empty, text after 300 ms",
      "aria-live assertive, textContent", "status with p", "status aria-live polite", "aria-live polite, text of a span changed",
      "log with p", "status aria-atomic with text and button",
    ] as const;
    for (const [n, form] of forms.entries()) {
      const text = `Live notice number ${n + 1}`;
      const heard = await said(
        () =>
          b.execute(
            async (form: string, text: string) => {
              const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
              const box = document.createElement("div");
              box.id = "control-live";
              document.body.append(box);
              const make = (tag: string, attrs: Record<string, string>, parent: Element = box): HTMLElement => {
                const el = document.createElement(tag);
                for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
                parent.append(el);
                return el;
              };
              const withButton = (el: HTMLElement) => {
                el.append(text);
                make("button", {}, el).textContent = "Dismiss";
              };
              if (form === "alert present, textContent") {
                const el = make("div", { role: "alert" });
                await wait(500);
                el.textContent = text;
              } else if (form === "alert new with text") make("div", { role: "alert" }).textContent = text;
              else if (form === "alert new with text and button") withButton(make("div", { role: "alert" }));
              else if (form === "alert empty, text after 300 ms") {
                const el = make("div", { role: "alert" });
                await wait(300);
                el.textContent = text;
              } else if (form === "aria-live assertive, textContent") {
                const el = make("div", { "aria-live": "assertive" });
                await wait(500);
                el.textContent = text;
              } else if (form === "status with p") {
                const el = make("div", { role: "status" });
                await wait(500);
                make("p", {}, el).textContent = text;
              } else if (form === "status aria-live polite") {
                const el = make("div", { role: "status", "aria-live": "polite" });
                await wait(500);
                el.textContent = text;
              } else if (form === "aria-live polite, text of a span changed") {
                const el = make("div", { "aria-live": "polite" });
                const span = make("span", {}, el);
                span.textContent = "before";
                await wait(500);
                span.textContent = text;
              } else if (form === "log with p") {
                const el = make("div", { role: "log" });
                await wait(500);
                make("p", {}, el).textContent = text;
              } else {
                const el = make("div", { role: "status", "aria-atomic": "true" });
                await wait(500);
                withButton(make("div", {}, el));
              }
            },
            form,
            text,
          ),
        2500,
      );
      await b.execute(() => document.getElementById("control-live")?.remove());
      record(`control: live region, ${form} (Orca ${heard.includes(text) ? "says" : "does not say"} it)`, heard ? heard.split(" | ") : []);
      expect(heard, `${form}: Orca said ${JSON.stringify(heard)}`).not.toContain(text);
    }
    // The silence above means something only if Orca is still speaking: a bare button that takes the focus is heard.
    const heardButton = await said(
      () =>
        b.execute(() => {
          const button = document.createElement("button");
          button.id = "control-live";
          button.textContent = "Probe button";
          document.body.append(button);
          button.focus();
        }),
      1500,
    );
    await b.execute(() => document.getElementById("control-live")?.remove());
    expect(heardButton, "Orca still speaks after the live regions").toContain("Probe button");
  });

  it("X3_no_csp_violation_in_the_shell_tour", async () => {
    const violations = await b.execute(() => (window as unknown as { __csp: string[] }).__csp);
    expect(violations).toEqual([]);
  });
});
