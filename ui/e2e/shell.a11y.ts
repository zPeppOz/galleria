// Plan B2 T7, the shell under a screen reader (Q22 (a): Orca and the AT-SPI tree in the real window, run locally by
// lab/tests/gui-a11y.sh, i.e. `scripts/gui.sh a11y`). One session of the app, in order: the list, the tabs, a text field,
// a switch, a notice that goes away by itself (and the confirmation of «Delete»). Assertions are on our own names and on the states
// AT-SPI reports; the words Orca uses for the role and the state are recorded in target/gui-a11y/transcript.txt and
// copied into docs/08-esito-fase2.md, because they are Orca's, not ours.
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  atspiTree, closeApp, daemon, descendants, findNode, focusedNode, openApp, orcaDebugLines, record, speechMark, speechSince, waitForBody, waitForSpeech, waitForTree,
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
    await waitForTree(focusedIs("Import…"), "the Import button focused");
    await key("Tab");
    await waitForTree(focusedIs("Menu"), "the Menu button focused");
    // Menu → the search field → the sort order → the list: Tab until the chosen row has the focus
    const tree = await reach("Ufficio Napoli", ["Tab"], 8);
    const listbox = findNode(tree, "list box", "Connections");
    expect(listbox, "a list box named «Connections»").toBeTruthy();
    const rows = descendants(tree, listbox as AtNode).filter((n) => n.role === "list item");
    // the order of the list: the groups of the seed, in each the favourites first and then by name
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
    // «Manage groups» (the foot of the column of the list), the star (a toggle button: «pressed» when the profile is a
    // favourite), then «More actions»
    await key("Tab");
    await waitForTree(focusedIs("Manage groups"), "«Manage groups» focused");
    await key("Tab");
    const star = await waitForTree(focusedIs("Favourite"), "the star focused");
    expect((focusedNode(star) as AtNode).role, "the star is a toggle button").toBe("toggle button");
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

  // KNOWN GAP OF THIS IMAGE ONLY (docs/08-esito-fase2.md, «Buco 1» and «Spike Orca»): the tree is right (entry «Name»,
  // labelled-by) and Orca 46.1 in the image says the value and «selected», not the name. On a real GNOME session Orca
  // 50.3 (Ubuntu 26.04) and 51.1 (Fedora 45) do say «Name» «entry», so this is not a gap of the product there. Only the
  // wait for the speech is turned upside down: the focus and the tree are asserted as usual, so a failure to get there
  // is a failure, not the gap. The day the image gets an Orca that says «Name» the last line fails, and the test turns
  // into a positive one.
  it("S4b_text_field_name_is_spoken_by_orca", async () => {
    const mark = speechMark();
    const tree = await reach("Name", ["Tab"]);
    const field = focusedNode(tree) as AtNode;
    expect(field.role).toBe("entry");
    expect(field.relations.some((r) => r.startsWith("labelled-by"))).toBe(true);
    await expect(waitForSpeech(mark, /\bName\b/, 2500), "known gap of the image (Orca 46.1): the name of a text field is not said; on real GNOME (Orca 50.3, 51.1) it is").rejects.toThrow(/Orca never said/);
    record("S4b text field reached (Orca 46.1 in the image does not say the name)", speechSince(mark), tree);
  });

  // T5: the switch of «General» is real now and writes at once (P7); the check of the switch under Orca that came out of
  // B2 with the demo form is back, on the real one. It turns «Reconnect automatically» off and on again, so the profile
  // of the seed ends as it began.
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

  // T8 brought the confirmation of «Delete» back (it was skipped from T3): the real alert dialog of the real profile, under
  // Orca. Escape gives up, so the profile stays and the session goes on as it was.
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
    const text = "The profile and its secrets are removed from this computer. This cannot be undone.";
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

  // The text of the newest notice in the page, whatever it is: «Duplicate» makes a real copy now, so the sentence has the
  // name of the copy in it and is read from the page, not written here.
  const lastNotice = async (): Promise<string> =>
    (await b.execute(() => {
      const all = [...document.querySelectorAll(".gx-toasts .gx-toast-message")];
      return all.length ? (all[all.length - 1].textContent ?? "") : "";
    })) as string;
  let firstCopy = "";

  it("S6_notice_appears_without_moving_the_focus", async () => {
    await reach("More actions", ["Shift", "Tab"], 20);
    await key("Enter");
    await waitForTree(focusedIs("Duplicate"), "the menu open on «Duplicate»");
    const mark = speechMark();
    await key("Enter");
    // the notice is in a live region that was already in the page and empty: it appears, the focus stays on the button
    // (the copy is chosen and the focus goes to «More actions» of the copy)
    const tree = await waitForTree((t) => !!findNode(t, "push button", "Dismiss the notice") && focusedIs("More actions")(t), "the notice shown and the focus on «More actions»");
    expect(findNode(tree, "push button", "Dismiss the notice")?.states.has("focused") ?? false).toBe(false);
    const region = tree.find((n) => n.role === "status bar");
    expect(region, "the live region is a status bar in the tree").toBeTruthy();
    firstCopy = await lastNotice();
    expect(firstCopy).toMatch(/^Duplicated as “Datacenter Milano \(copy\)”\.$/);
    // what Orca said in the meantime, whatever it is, goes into the transcript
    await new Promise((resolve) => setTimeout(resolve, 3000));
    record("S6 notice after «Duplicate»", speechSince(mark), tree);
  });

  // Q27 (spec §12.1): the live regions of WebKitGTK stay mute under Orca (the controls below say so), so the notice is
  // also sent by the `announce` command, which makes the Rust side emit the ATK `notification` signal on the accessible
  // of the window; the atk-bridge turns it into `object:announcement` and Orca says it. S6b is the positive test the
  // spike called for: after «Duplicate» Orca says the text of the notice, once, and the focus stays on «More actions».
  let NOTICE = "";
  const noticeShown = (t: AtNode[]) => !!findNode(t, "push button", "Dismiss the notice") && focusedIs("More actions")(t);

  it("S6b_notice_is_spoken_by_orca", async () => {
    await key("Enter");
    await waitForTree(focusedIs("Duplicate"), "the menu open on «Duplicate»");
    const mark = speechMark();
    await key("Enter");
    // the notice of the first copy may still be in the page: wait for the one of this copy
    await b.waitUntil(async () => (await lastNotice()) !== firstCopy, { timeout: 8000, interval: 200, timeoutMsg: "the notice of the second copy never showed" });
    NOTICE = await lastNotice();
    expect(NOTICE).toMatch(/^Duplicated as “.+”\.$/);
    const tree = await waitForTree(noticeShown, "the notice shown and the focus on «More actions»");
    const said = await waitForSpeech(mark, NOTICE, 4000);
    // once: the notice is in the page too, and if Orca ever starts to say the live region this counts two
    await new Promise((resolve) => setTimeout(resolve, 1500));
    const times = speechSince(mark).filter((line) => line.includes(NOTICE)).length;
    record("S6b notice after «Duplicate», spoken by Orca", speechSince(mark), tree);
    expect(times, `Orca said the notice ${times} times: ${JSON.stringify(speechSince(mark))}`).toBe(1);
    expect(said.some((line) => line.includes(NOTICE))).toBe(true);
    // the announcement does not take the focus
    expect(focusedIs("More actions")(atspiTree())).toBe(true);
  });

  // S20: the signal comes from the galleria-gui process, not from a stand-in. Orca's debug file has one line per
  // `object:announcement` it dequeues, with the source and the application.
  it("S6c_the_announcement_comes_from_the_app", async () => {
    const lines = orcaDebugLines(/Dequeued object:announcement/).filter((line) => line.includes(NOTICE));
    record("S6c object:announcement lines for the notice", lines);
    expect(lines.length, "Orca dequeued an object:announcement carrying the notice").toBeGreaterThan(0);
    for (const line of lines) expect(line).toMatch(/for \[frame: 'Galleria'\] in \[application: 'galleria-gui'\]/);
  });

  // Controls for the two gaps, in the style of `control_how_webkitgtk_exposes_a_plain_popup_button`: bare elements put
  // in the page, nothing of ours. They assert what Orca does today in this image, so the day it changes the run says so.
  // The text field gap is of the image only (Orca 46.1); the live region gap is also there on real GNOME (docs/08,
  // «Spike Orca»).
  const settle = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  async function said(act: () => Promise<unknown>, ms: number): Promise<string> {
    const mark = speechMark();
    await act();
    await settle(ms);
    return speechSince(mark).join(" | ");
  }

  it("control_orca_does_not_say_the_name_of_a_bare_text_field", async () => {
    // One field at a time, reached with Tab from a bare button, in nine forms. The tree has the right name for every
    // one of them. Without anything of ours Orca 46.1 says the value and «selected» (or «Text unselected.»), not the name.
    // Image only: Orca 50.3 and 51.1 on real GNOME say the name of our fields (bare fields were not tried there).
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
      // The empty field and the textarea are recorded and not asserted: Orca says the name of the empty field on arrival
      // in some runs («Probe alpha entry.», «Probe alpha empty entry.» in the review) and nothing at all in others, and
      // the same goes for the textarea («Probe foxtrot entry Some value.»: 2 runs of 4 with the announcement of Q27, 1 of 3 with
      // it switched off in the A/B of B3-T2, so it is the dispatch of Orca, not ours). The seven other
      // fields with a value are the gap: the name is never said.
      if (form !== "aria-label empty" && form !== "textarea") expect(names, `${form}: Orca said ${JSON.stringify(heard)}`).toBe(false);
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
