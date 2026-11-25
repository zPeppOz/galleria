/// <reference types="node" />
// T7 (plan B2): the whole shell used with the keyboard only. These tests never touch the pointer: `user.tab()`,
// `user.keyboard()` and nothing else. jsdom has no layout and no screen reader, so what is proved here is the order of
// the stops, the reach of every control and where the focus goes after each close; the window itself is read with
// Orca in `ui/e2e/shell.a11y.ts`.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { mockIPC } from "@tauri-apps/api/mocks";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { DaemonView } from "../daemon/types";
import { makeView } from "../test/fixtures";
import { renderShell } from "../test/shell";

type User = ReturnType<typeof userEvent.setup>;

const show = (view: DaemonView | null = makeView(), onRetry = () => undefined) =>
  renderShell(view, { onRetry });

const firstRun = () => makeView({ link: "daemon_not_running", status: null });

/** The live region of the notices (the search count is a `status` too). */
const toastStatus = (): HTMLElement => document.querySelector(".gx-toasts [role=\"status\"]") as HTMLElement;

/** What a screen reader would call the element. */
function nameOf(el: Element): string {
  const labelledBy = el.getAttribute("aria-labelledby");
  if (labelledBy) return labelledBy.split(" ").map((id) => document.getElementById(id)?.textContent ?? "").join(" ").trim();
  const label = el.getAttribute("aria-label");
  if (label) return label;
  const id = el.getAttribute("id");
  const forLabel = id ? document.querySelector(`label[for="${CSS.escape(id)}"]`) : null;
  if (forLabel) return forLabel.textContent?.trim() ?? "";
  const wrapping = el.closest("label");
  if (wrapping) return wrapping.querySelector(".gx-switch-label")?.textContent?.trim() ?? wrapping.textContent?.trim() ?? "";
  return (el.textContent ?? "").trim();
}

/** Waits for a menu to be open and for the focus to be on one of its entries: a key pressed earlier goes to the button. */
async function menuOpen(name: string): Promise<HTMLElement> {
  const menu = await screen.findByRole("menu", { name });
  await waitFor(() => expect(menu.contains(document.activeElement) && document.activeElement !== menu).toBe(true));
  return menu;
}

const roleOf = (el: Element): string => el.getAttribute("role") ?? el.tagName.toLowerCase();

/** The elements that Tab stops on, in order, from the top of the document until the focus leaves it or comes round. */
async function tabStops(user: User): Promise<HTMLElement[]> {
  const stops: HTMLElement[] = [];
  (document.activeElement as HTMLElement | null)?.blur();
  for (let step = 0; step < 60; step++) {
    await user.tab();
    const now = document.activeElement as HTMLElement;
    if (now === document.body || stops.includes(now)) return stops;
    stops.push(now);
  }
  throw new Error(`the Tab key never left the document: ${stops.map(nameOf).join(" → ")}`);
}

const INTERACTIVE = 'button, input, select, textarea, a[href], [role="option"], [role="tab"], [role="switch"], [role="menuitem"], [tabindex]';

/** Every element a person could operate in the current document (leaving out what is hidden, disabled or only a focus guard). */
function interactiveElements(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>(INTERACTIVE)].filter(
    (el) =>
      !el.closest("[hidden], [aria-hidden='true'], [inert]") &&
      !(el as HTMLButtonElement).disabled &&
      el.getAttribute("type") !== "hidden" &&
      // Base UI puts two empty guards around a dialog; they are not controls
      !el.hasAttribute("data-base-ui-focus-guard") &&
      !(el.getAttribute("tabindex") === "-1" && !el.getAttribute("role")),
  );
}

describe("keyboard on the whole shell", () => {
  it("tab_order_follows_the_layout", async () => {
    const user = userEvent.setup();
    show(firstRun());
    const stops = await tabStops(user);
    const seen = stops.map((el) => `${roleOf(el)}:${nameOf(el)}`);
    // The menu button, the search field, the sort order (one stop: the chosen radio), the list (one stop: the chosen row),
    // «Manage groups» (the foot of the column), the first-run panel («Copy», «Try again»), the star, «More actions», the tab
    // strip (one stop: the chosen tab) and then the panel itself and its first control.
    expect(seen.slice(0, 9)).toEqual(["button:Menu", "input:Search connections", "input:By name", "option:Ufficio Napoli", "button:Manage groups", "button:Copy", "button:Try again", "button:Favourite", "button:More actions"]);
    expect(seen[9]).toBe("tab:General");
    const rest = seen.slice(10);
    // After the strip comes the panel of the chosen tab (Base UI makes it a stop) or straight its first control; the
    // first control is the name field either way.
    expect(rest.some((s) => s === "input:Name")).toBe(true);
    expect(rest.indexOf("input:Name")).toBeLessThanOrEqual(1);
  });

  it("tab_order_without_the_first_run_panel_skips_its_buttons", async () => {
    const user = userEvent.setup();
    show();
    const seen = (await tabStops(user)).map((el) => `${roleOf(el)}:${nameOf(el)}`);
    expect(seen.slice(0, 8)).toEqual(["button:Menu", "input:Search connections", "input:By name", "option:Ufficio Napoli", "button:Manage groups", "button:Favourite", "button:More actions", "tab:General"]);
    expect(seen).not.toContain("button:Copy");
  });

  it("every_interactive_element_is_reachable", async () => {
    const user = userEvent.setup();
    // the arrows on the sort order write `state.toml`: here the app answers, so no notice comes of it
    mockIPC(() => undefined);
    // «Ufficio Napoli» is OpenVPN: its «General» tab has the transport choice (radios) as well.
    show(firstRun());
    const key = (el: Element) => `${roleOf(el)}:${nameOf(el)}`;
    // by name and role: choosing a tab draws a new panel, so the same control is a new element
    const reached = new Set((await tabStops(user)).map(key));
    // Arrows reach the rest of a composite widget: the other rows of the list, the other tabs, the other radios.
    const listbox = screen.getByRole("listbox");
    const rows = within(listbox).getAllByRole("option");
    rows[0].focus();
    for (let i = 0; i < rows.length; i++) {
      reached.add(key(document.activeElement as HTMLElement));
      await user.keyboard("{ArrowDown}");
    }
    reached.add(key(document.activeElement as HTMLElement));
    // the chosen row now is the last; go back to the first so the rest of the test starts as it did
    await user.keyboard("{Home}");
    const tabs = screen.getAllByRole("tab");
    screen.getByRole("tab", { name: "General" }).focus();
    for (let i = 0; i < tabs.length; i++) {
      reached.add(key(document.activeElement as HTMLElement));
      await user.keyboard("{ArrowRight}");
    }
    reached.add(key(document.activeElement as HTMLElement));
    await user.keyboard("{Home}");
    // each group of radios on its own: the sort order and the transport
    for (const group of screen.getAllByRole("radiogroup")) {
      const radios = within(group).getAllByRole("radio");
      radios[0].focus();
      for (let i = 0; i < radios.length; i++) {
        reached.add(key(document.activeElement as HTMLElement));
        await user.keyboard("{ArrowRight}");
      }
      reached.add(key(document.activeElement as HTMLElement));
    }

    const missed = interactiveElements().filter((el) => !reached.has(key(el)));
    expect(missed.map((el) => `${roleOf(el)}:${nameOf(el)}`)).toEqual([]);
    expect(interactiveElements().length).toBeGreaterThan(12);
  });

  it("every_control_of_every_tab_is_reachable_for_both_protocols", async () => {
    const user = userEvent.setup();
    show();
    for (const profile of ["Homelab", "Cliente Rossi"]) {
      screen.getByRole("option", { name: profile }).focus();
      await user.keyboard("{Enter}");
      const stops = new Set(await tabStops(user));
      const detail = screen.getByRole("main");
      const controls = [...detail.querySelectorAll<HTMLElement>("input, select, [role='switch']")].filter((el) => el.getAttribute("type") !== "hidden");
      expect(controls.length).toBeGreaterThan(2);
      const radios = controls.filter((el) => el.getAttribute("type") === "radio");
      for (const control of controls.filter((el) => !radios.includes(el))) {
        expect(stops.has(control), `${profile}: ${roleOf(control)} «${nameOf(control)}» is a stop`).toBe(true);
      }
      // the radio group is one stop (the chosen radio)
      if (radios.length) expect(radios.filter((el) => stops.has(el))).toHaveLength(1);
    }
  });

  it("focus_is_never_lost_to_the_body", async () => {
    const user = userEvent.setup();
    show();
    const active = () => document.activeElement as HTMLElement;
    const here = (name: string) => expect(`${roleOf(active())}:${nameOf(active())}`, "where the focus is").toBe(name);

    // the app menu: Escape, then a choice
    await user.tab();
    here("button:Menu");
    await user.keyboard("{Enter}");
    await menuOpen("Menu");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    here("button:Menu");
    await user.keyboard("{Enter}");
    await menuOpen("Menu");
    await user.keyboard("{ArrowDown}{Enter}"); // «English», already the language: the menu closes
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    here("button:Menu");

    // the About dialog: opened from the menu, closed with Escape, back on «Menu»
    await user.keyboard("{Enter}");
    await menuOpen("Menu");
    await user.keyboard("{End}{Enter}");
    await screen.findByRole("dialog", { name: "About Galleria" });
    expect(active()).not.toBe(document.body);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    here("button:Menu");

    // «More actions» → Delete asks first; Escape gives up and the focus goes back to the button
    screen.getByRole("button", { name: "More actions" }).focus();
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{End}{Enter}");
    await screen.findByRole("alertdialog", { name: "Delete Ufficio Napoli?" });
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    here("button:More actions");
    // «Duplicate» closes the menu, makes the copy and puts the focus on «More actions» of the copy
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{Home}{Enter}");
    await waitFor(() => expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Ufficio Napoli (copy)"));
    await waitFor(() => here("button:More actions"));
  });

  it("a_notice_closed_from_the_keyboard_does_not_drop_the_focus_on_the_body", async () => {
    const user = userEvent.setup();
    show();
    const more = screen.getByRole("button", { name: "More actions" });
    more.focus();
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{Home}{Enter}");
    const status = toastStatus();
    await waitFor(() => expect(status.textContent).toContain("Duplicated as"));
    // the copy is chosen, so the form is another one: the focus is on its «More actions»
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "More actions" })));
    const here = screen.getByRole("button", { name: "More actions" });
    // a person who wants to read it or close it gets there with Tab (it comes after the main area)
    const close = within(status).getByRole("button", { name: "Dismiss the notice" });
    close.focus();
    await user.keyboard("{Enter}");
    await waitFor(() => expect(within(status).queryByRole("button")).toBeNull());
    expect(document.activeElement, "the focus after the notice left").not.toBe(document.body);
    expect(document.activeElement).toBe(here);
  });

  it("a_full_tour_needs_no_pointer", async () => {
    const user = userEvent.setup();
    show();
    const active = () => document.activeElement as HTMLElement;
    const title = () => screen.getByRole("heading", { level: 1 }).textContent;
    // a profile: Menu → search → sort order → list → the arrows move the choice
    await user.tab();
    await user.tab();
    await user.tab();
    await user.tab();
    expect(nameOf(active())).toBe("Ufficio Napoli");
    expect(nameOf(active())).toBe("Ufficio Napoli");
    await user.keyboard("{ArrowDown}");
    expect(title()).toBe("Datacenter Milano");
    await user.keyboard("{End}");
    expect(title()).toBe("Cliente Rossi");
    await user.keyboard("{Home}");
    expect(title()).toBe("Ufficio Napoli");

    // a tab: to the strip, the arrows choose; the panel shows
    await user.tab(); // «Manage groups»
    await user.tab(); // the star
    await user.tab(); // More actions
    await user.tab(); // the tab strip
    expect(roleOf(active())).toBe("tab");
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Authentication" }).getAttribute("aria-selected")).toBe("true");
    await user.keyboard("{End}");
    expect(screen.getByRole("tab", { name: "Log" }).getAttribute("aria-selected")).toBe("true");
    await user.keyboard("{Home}");
    expect(screen.getByRole("tab", { name: "General" }).getAttribute("aria-selected")).toBe("true");

    // the fields are stops of the panel; the name can be typed in, and then «Save» and «Cancel» are two more stops
    for (let step = 0; step < 12 && !(active() instanceof HTMLInputElement); step++) await user.tab();
    expect(nameOf(active())).toBe("Name");
    expect((active() as HTMLInputElement).readOnly).toBe(false);
    await user.keyboard("x");
    // a field that gets the focus with Tab has its text selected: the key replaces it
    expect((active() as HTMLInputElement).value).toBe("x");
    const bar = screen.getByRole("group", { name: "Unsaved changes" });
    screen.getByRole("button", { name: "Cancel" }).focus();
    await user.keyboard("{Enter}");
    expect(screen.queryByRole("group", { name: "Unsaved changes" })).toBeNull();
    expect(bar.isConnected).toBe(false);
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Ufficio Napoli");
    expect(active(), "the focus after Cancel").toBe(screen.getByRole("tab", { name: "General" }));

    // «Delete» asks first; Escape gives up and nothing changes
    screen.getByRole("button", { name: "More actions" }).focus();
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    await screen.findByRole("alertdialog", { name: "Delete Ufficio Napoli?" });
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(5);

    // the language and the theme from the menu
    screen.getByRole("button", { name: "Menu" }).focus();
    await user.keyboard("{Enter}");
    await menuOpen("Menu");
    await user.keyboard("{Home}{ArrowDown}{ArrowDown}{Enter}"); // Italiano
    await waitFor(() => expect(screen.getByRole("tab", { name: "Generale" })).toBeTruthy());
    expect(document.documentElement.lang).toBe("it");
    expect(active()).toBe(screen.getByRole("button", { name: "Menu" }));
    await user.keyboard("{Enter}");
    await menuOpen("Menu");
    await user.keyboard("{Home}{ArrowDown}{ArrowDown}{ArrowDown}{ArrowDown}{ArrowDown}{Enter}"); // Scuro
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("dark"));
    await user.keyboard("{Enter}");
    await menuOpen("Menu");
    await user.keyboard("{Home}{ArrowDown}{ArrowDown}{ArrowDown}{ArrowDown}{Enter}"); // Chiaro
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
  });
});

// ---- the focus ring --------------------------------------------------------------------------------------------

const SRC = join(import.meta.dirname, "..");
const cssFiles = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? cssFiles(path) : name.endsWith(".css") ? [path] : [];
  });

const stripped = (file: string) => readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
const rules = (file: string) => [...stripped(file).matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((m) => ({ selector: m[1].trim(), body: m[2] }));

describe("focus ring", () => {
  it("focus_ring_rule_exists_for_every_control", () => {
    const all = cssFiles(SRC).flatMap((file) => rules(file).map((r) => ({ file, ...r })));
    // One rule for everything that takes the focus...
    const global = all.find((r) => r.selector === ":focus-visible");
    expect(global?.body).toMatch(/outline:\s*var\(--focus-ring\)/);
    // ...so any rule that removes an outline must put one back for the same element, or hide something that is not a stop.
    const removes = all.filter((r) => /outline:\s*(none|0)\b/.test(r.body));
    const NOT_A_STOP = [".gx-menu-positioner", ".gx-menu"]; // containers: the focus is on the item
    for (const rule of removes) {
      if (NOT_A_STOP.includes(rule.selector)) continue;
      // `.gx-menu-item`: the highlighted one draws the ring itself
      const selectors = rule.selector.split(",").map((s) => s.trim());
      for (const selector of selectors) {
        const restored = all.some((r) => r.selector.startsWith(selector) && /\[data-highlighted\]|:focus-visible/.test(r.selector) && /outline:\s*var\(--focus-ring\)/.test(r.body));
        expect(restored, `${selector} removes its outline and does not draw a ring of its own`).toBe(true);
      }
    }
    // Nothing draws its own outline or box-shadow ring for the focus with a colour of its own.
    const ownRing = all.filter((r) => /:focus(-visible|-within)?\b/.test(r.selector) && /outline:/.test(r.body) && !/var\(--focus-ring\)/.test(r.body));
    expect(ownRing.map((r) => `${r.file}: ${r.selector}`)).toEqual([]);
  });

  it("the_segmented_radio_is_hidden_with_opacity_so_its_label_draws_the_ring", () => {
    // The radio itself is invisible (opacity 0) and still a stop: the ring is drawn on the label that follows it.
    const segmented = rules(join(SRC, "design", "Segmented.css"));
    expect(segmented.some((r) => /input:focus-visible \+ \.gx-segmented-label/.test(r.selector) && /var\(--focus-ring\)/.test(r.body))).toBe(true);
  });
});
