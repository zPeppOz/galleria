/// <reference types="node" />
// T7 (plan B2): the whole shell used with the keyboard only. These tests never touch the pointer: `user.tab()`,
// `user.keyboard()` and nothing else. jsdom has no layout and no screen reader, so what is proved here is the order of
// the stops, the reach of every control and where the focus goes after each close; the window itself is read with
// Orca in `ui/e2e/shell.a11y.ts`.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { DaemonView } from "../daemon/types";
import type { ImportActions } from "../import/types";
import type { CmdError, ProfilePatch, ProfileSummary } from "../rubrica/types";
import { applyPatch, makeDetail, makePreview, makeStatus, makeSummary, makeView, SAMPLE_PROFILES } from "../test/fixtures";
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
    // «Import…», the menu button, the search field, the sort order (one stop: the chosen radio), the list (one stop: the chosen row),
    // «Manage groups» (the foot of the column), the first-run panel («Copy», «Try again»), the star, «More actions», the tab
    // strip (one stop: the chosen tab) and then the panel itself and its first control.
    expect(seen.slice(0, 10)).toEqual(["button:Import…", "button:Menu", "input:Search connections", "input:By name", "option:Ufficio Napoli", "button:Manage groups", "button:Copy", "button:Try again", "button:Favourite", "button:More actions"]);
    expect(seen[10]).toBe("tab:General");
    const rest = seen.slice(11);
    // After the strip comes the panel of the chosen tab (Base UI makes it a stop) or straight its first control; the
    // first control is the name field either way.
    expect(rest.some((s) => s === "input:Name")).toBe(true);
    expect(rest.indexOf("input:Name")).toBeLessThanOrEqual(1);
  });

  it("tab_order_without_the_first_run_panel_skips_its_buttons", async () => {
    const user = userEvent.setup();
    show();
    const seen = (await tabStops(user)).map((el) => `${roleOf(el)}:${nameOf(el)}`);
    expect(seen.slice(0, 10)).toEqual(["button:Import…", "button:Menu", "input:Search connections", "input:By name", "option:Ufficio Napoli", "button:Manage groups", "button:Favourite", "button:Connect", "button:More actions", "tab:General"]);
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
    await user.tab(); // «Import…» comes first
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

  it("a_connection_needs_no_pointer", async () => {
    // select a row, Enter connects, the state comes with the events, Enter again disconnects
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => void calls.push({ cmd, args }), { shouldMockEvents: true });
    const user = userEvent.setup();
    const first = renderShell(makeView());
    const row = () => screen.getByRole("option", { name: /Ufficio Napoli/ });
    row().focus();
    await user.keyboard("{Enter}");
    await waitFor(() => expect(calls.filter((c) => c.cmd === "connect_profile")).toEqual([{ cmd: "connect_profile", args: { id: "office", replace: false } }]));
    // while the answer has not been followed by an event, a second Enter asks for nothing
    await user.keyboard("{Enter}");
    expect(calls.filter((c) => c.cmd === "connect_profile" || c.cmd === "disconnect")).toHaveLength(1);
    first.unmount();
    // the session is up (the event came): Enter on the same row ends it
    renderShell(makeView({ status: makeStatus({ state: "connected", session: 1, profile: { id: "office", name: "Ufficio Napoli" } }) }));
    row().focus();
    await user.keyboard("{Enter}");
    await waitFor(() => expect(calls.some((c) => c.cmd === "disconnect")).toBe(true));
    // a row that is not the chosen one is chosen by Enter and nothing else happens
    const before = calls.length;
    screen.getByRole("option", { name: /Homelab/ }).focus();
    await user.keyboard("{Enter}");
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Homelab");
    expect(calls.length).toBe(before);
  });

  it("a_full_tour_needs_no_pointer", async () => {
    const user = userEvent.setup();
    show();
    const active = () => document.activeElement as HTMLElement;
    const title = () => screen.getByRole("heading", { level: 1 }).textContent;
    // a profile: Import… → Menu → search → sort order → list → the arrows move the choice
    await user.tab();
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
    await user.tab(); // Connect
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

  it("a_full_rubrica_tour_needs_no_pointer", async () => {
    const user = userEvent.setup();
    const announce = vi.fn();
    renderShell(makeView(), { announce });
    const active = () => document.activeElement as HTMLElement;
    const title = () => screen.getByRole("heading", { level: 1 }).textContent;
    const noticeSaid = (text: string) => waitFor(() => expect(announce).toHaveBeenCalledWith(text, false));
    /** Tab until the focus is on the control with this role and name (a test that never presses a pointer key). */
    const reach = async (role: string, name: string) => {
      for (let step = 0; step < 40 && `${roleOf(active())}:${nameOf(active())}` !== `${role}:${name}`; step++) await user.tab();
      expect(`${roleOf(active())}:${nameOf(active())}`, "where the focus is").toBe(`${role}:${name}`);
    };

    // 1 · search: Ctrl+K from anywhere, a few letters, the arrow goes down to the list, and the choice is the result
    await user.keyboard("{Control>}k{/Control}");
    expect(active()).toBe(screen.getByRole("searchbox", { name: "Search connections" }));
    await user.keyboard("milano");
    await waitFor(() => expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(1));
    await user.keyboard("{ArrowDown}");
    expect(roleOf(active())).toBe("option");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(title()).toBe("Datacenter Milano"));
    // Escape in the field empties it: the whole list is back
    await user.keyboard("{Control>}k{/Control}{Escape}");
    await waitFor(() => expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(5));

    // 2 · edit and save: the name field, a letter, Ctrl+S; the notice is said and the focus stays on the field
    await reach("input", "Name");
    await user.keyboard("{End}x");
    await screen.findByRole("group", { name: "Unsaved changes" });
    await user.keyboard("{Control>}s{/Control}");
    await noticeSaid("Profile saved.");
    expect(title()).toBe("Datacenter Milanox");
    expect(nameOf(active())).toBe("Name");

    // 3 · another group: the field of the group is a native choice, which jsdom does not drive with keys: the value is
    // set the way the browser does when a key chooses an entry, and the save is again from the keyboard
    await reach("select", "Group");
    fireEvent.change(active(), { target: { value: "Personale" } });
    await user.keyboard("{Control>}s{/Control}");
    await waitFor(() => expect(announce).toHaveBeenCalledTimes(2));
    expect((screen.getByRole("combobox", { name: "Group" }) as HTMLSelectElement).value).toBe("Personale");

    // 4 · the raw editor: the strip, the arrows to «Configuration», the text, Ctrl+S
    await reach("tab", "General");
    await user.keyboard("{End}{ArrowLeft}");
    expect(screen.getByRole("tab", { name: "Configuration" }).getAttribute("aria-selected")).toBe("true");
    await reach("textarea", "Configuration");
    await user.keyboard("{Control>}{End}{/Control}# by hand");
    await screen.findByRole("group", { name: "Unsaved changes" });
    await user.keyboard("{Control>}s{/Control}");
    await waitFor(() => expect(announce).toHaveBeenCalledTimes(3));

    // 5 · «Manage groups»: the dialog opens, a group is added with Enter, Escape closes it and the focus is back
    await reach("button", "Manage groups");
    await user.keyboard("{Enter}");
    const dialog = await screen.findByRole("dialog", { name: "Manage groups" });
    await user.type(within(dialog).getByRole("textbox", { name: "New group" }), "Famiglia{Enter}");
    await noticeSaid("Group Famiglia created.");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(`${roleOf(active())}:${nameOf(active())}`).toBe("button:Manage groups");

    // 6 · duplicate and delete, from the menu «More actions»; the delete asks and takes the answer from the keyboard
    await reach("button", "More actions");
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{Home}{Enter}");
    await waitFor(() => expect(title()).toBe("Datacenter Milano (copy)"));
    await waitFor(() => expect(nameOf(active())).toBe("More actions"));
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{End}{Enter}");
    const confirm = await screen.findByRole("alertdialog", { name: "Delete Datacenter Milano (copy)?" });
    // the focus is on the safe answer; Tab reaches the other, and the page behind is out of reach
    expect(nameOf(active())).toBe("Cancel");
    await user.tab();
    expect(nameOf(active())).toBe("Delete");
    expect(confirm.contains(active())).toBe(true);
    await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    await noticeSaid("Deleted “Datacenter Milano (copy)”.");
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(5);
    expect(roleOf(active()), "the focus after the delete is in the list").toBe("option");
  });

  it("an_import_from_text_needs_no_pointer", async () => {
    const user = userEvent.setup();
    const announce = vi.fn();
    renderShell(makeView(), { announce });
    const active = () => document.activeElement as HTMLElement;
    const reach = async (role: string, name: string) => {
      for (let step = 0; step < 20 && `${roleOf(active())}:${nameOf(active())}` !== `${role}:${name}`; step++) await user.tab();
      expect(`${roleOf(active())}:${nameOf(active())}`, "where the focus is").toBe(`${role}:${name}`);
    };
    // Ctrl+O opens the window and the focus is on the text
    await user.keyboard("{Control>}o{/Control}");
    await screen.findByRole("dialog", { name: "Import profile" });
    await waitFor(() => expect(`${roleOf(active())}:${nameOf(active())}`).toBe("textarea:Paste the text of the profile"));
    await user.keyboard("client");
    // Tab goes to «Preview», Enter on it makes the preview and the focus goes to the name
    await reach("button", "Preview");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(`${roleOf(active())}:${nameOf(active())}`).toBe("input:Name"));
    // the name is written, the group is a stop of the Tab key, and «Import» is the next one
    await user.clear(active());
    await user.keyboard("Casa");
    await reach("select", "Group");
    await reach("button", "Import");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(announce).toHaveBeenCalledWith("Imported “Casa”.", false));
    // the new profile is the chosen one and has the focus: the arrows go on from there
    await waitFor(() => expect(`${roleOf(active())}:${nameOf(active())}`).toMatch(/^option:Casa/));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Casa");
  });

  it("a_full_import_tour_needs_no_pointer", async () => {
    const user = userEvent.setup();
    const announce = vi.fn();
    renderShell(makeView(), { announce });
    const active = () => document.activeElement as HTMLElement;
    const here = () => `${roleOf(active())}:${nameOf(active())}`;
    const reach = async (role: string, name: string) => {
      for (let step = 0; step < 20 && here() !== `${role}:${name}`; step++) await user.tab();
      expect(here(), "where the focus is").toBe(`${role}:${name}`);
    };
    // the text, the preview, a name of our own and a group, «Import»: the keys only
    await user.keyboard("{Control>}o{/Control}");
    await screen.findByRole("dialog", { name: "Import profile" });
    await waitFor(() => expect(here()).toBe("textarea:Paste the text of the profile"));
    await user.keyboard("client");
    await reach("button", "Preview");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(here()).toBe("input:Name"));
    await user.clear(active());
    await user.keyboard("Casa");
    await reach("select", "Group");
    // jsdom does not drive a native `select` with keys: the change is what the keys of a browser would make
    fireEvent.change(active(), { target: { value: "Lavoro" } });
    expect((active() as HTMLSelectElement).value).toBe("Lavoro");
    await reach("button", "Import");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(announce).toHaveBeenCalledWith("Imported “Casa”.", false));
    await waitFor(() => expect(here()).toMatch(/^option:Casa/));
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(6);

    // and once more: the file is chosen with the button (the window of Rust is a fake), then «Cancel» gives up
    await user.keyboard("{Control>}o{/Control}");
    await screen.findByRole("dialog", { name: "Import profile" });
    await waitFor(() => expect(here()).toBe("textarea:Paste the text of the profile"));
    await reach("button", "Choose a file…");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(here()).toBe("input:Name"));
    expect(screen.getByRole("dialog").textContent).toContain("File: separate.ovpn");
    await reach("button", "Cancel");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(active(), "the focus after «Cancel»").not.toBe(document.body);
    // nothing new in the list, and nothing said: giving up is not an outcome
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(6);
    expect(announce).toHaveBeenCalledTimes(1);
  });

  it("focus_is_never_lost_to_the_body_during_the_import", async () => {
    const notBody = (what: string) => expect(document.activeElement, `the focus after ${what}`).not.toBe(document.body);
    const refuse = (code: string, data: unknown = null) => async () => Promise.reject({ code, message: "x", data });
    const importer = (over: Partial<ImportActions> = {}): ImportActions => ({
      fromText: async () => makePreview(),
      pickFile: async () => null,
      commit: async (_id, name) => makeSummary({ id: "made", name }),
      discard: async () => undefined,
      ...over,
    });
    const open = async (user: ReturnType<typeof userEvent.setup>) => {
      await user.keyboard("{Control>}o{/Control}");
      await screen.findByRole("dialog", { name: "Import profile" });
      await waitFor(() => expect(document.activeElement?.tagName).toBe("TEXTAREA"));
    };
    const inTheAlert = () => expect(document.querySelector(".import-error")?.contains(document.activeElement), "the alert has the focus").toBe(true);

    // given up: Escape on the first step, and Escape on the preview
    let user = userEvent.setup();
    renderShell(makeView(), { importer: importer() });
    await open(user);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    notBody("giving up the first step");
    await open(user);
    await user.keyboard("client{Tab}{Tab}{Enter}");
    await waitFor(() => expect(document.activeElement?.getAttribute("name")).toBe("import-name"));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    notBody("giving up the preview");
    cleanup();

    // an error of the text: the alert has the focus; Escape closes the window and the focus is not lost
    user = userEvent.setup();
    renderShell(makeView(), { importer: importer({ fromText: refuse("import_syntax", { line: 2 }) }) });
    await open(user);
    await user.keyboard("client{Tab}{Tab}{Enter}");
    await waitFor(inTheAlert);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    notBody("an error of the text");
    cleanup();

    // a preview that ran out: the window goes back to the first step, says so and keeps the focus in it
    user = userEvent.setup();
    renderShell(makeView(), { importer: importer({ commit: refuse("gui_import_expired") }) });
    await open(user);
    await user.keyboard("client{Tab}{Tab}{Enter}");
    await waitFor(() => expect(document.activeElement?.getAttribute("name")).toBe("import-name"));
    await user.keyboard("{Enter}");
    await waitFor(inTheAlert);
    expect(screen.getByRole("textbox", { name: "Paste the text of the profile" }), "back at the first step").toBeTruthy();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    notBody("a preview that ran out");
    cleanup();

    // an offer that came from outside: the preview, then an error; each has the focus, and Escape leaves it somewhere
    user = userEvent.setup();
    renderShell(makeView(), { incoming: { offer: { preview: makePreview({ origin: "open", file_name: "inline.ovpn", suggested_name: "inline" }) }, turn: 1 } });
    await waitFor(() => expect(document.activeElement?.getAttribute("name")).toBe("import-name"));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    notBody("an offer from outside");
    cleanup();
    user = userEvent.setup();
    renderShell(makeView(), { incoming: { offer: { failed: { error: { code: "gui_import_unsupported_file", message: "x", data: { file_name: "a.txt" } } as CmdError, file_name: "a.txt" } }, turn: 1 } });
    await waitFor(inTheAlert);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    notBody("an error of a file from outside");
  });

  it("the_outcome_of_an_import_is_said_once_and_an_error_of_the_import_is_not_said", async () => {
    const user = userEvent.setup();
    const announce = vi.fn();
    const notice = (line: number, directive: string) => ({ line, directive, kind: "dangerous" as const, dns_script: false });
    let fail = false;
    const importer: ImportActions = {
      fromText: async () => {
        if (fail) throw { code: "import_syntax", message: "x", data: { line: 3 } };
        return makePreview({ suggested_name: "Malicious", removed: 2, notices: [notice(5, "up"), notice(6, "plugin")] });
      },
      pickFile: async () => null,
      commit: async (_id, name) => makeSummary({ id: "made", name }),
      discard: async () => undefined,
    };
    renderShell(makeView(), { announce, importer });
    const open = async () => {
      await user.keyboard("{Control>}o{/Control}");
      await screen.findByRole("dialog", { name: "Import profile" });
      await waitFor(() => expect(document.activeElement?.tagName).toBe("TEXTAREA"));
      await user.keyboard("client{Tab}{Tab}{Enter}");
    };
    await open();
    await waitFor(() => expect(document.activeElement?.getAttribute("name")).toBe("import-name"));
    await user.keyboard("{Enter}");
    // the text of the notice on the screen is the text that was announced, polite, and once
    await waitFor(() => expect(announce).toHaveBeenCalledWith("Imported “Malicious”. 2 directives removed.", false));
    expect(toastStatus().textContent).toContain("Imported “Malicious”. 2 directives removed.");
    // an error is read where the focus is: nothing is announced and no notice appears (the region stays there, empty)
    fail = true;
    await open();
    await waitFor(() => expect(document.querySelector(".import-error")?.contains(document.activeElement)).toBe(true));
    expect(announce).toHaveBeenCalledTimes(1);
    expect(document.querySelector(".gx-toasts [role=\"alert\"]")?.textContent).toBe("");
    expect(document.querySelectorAll(".gx-toast")).toHaveLength(1);
  });

  it("every_control_of_every_tab_and_of_the_new_dialogs_is_reachable", async () => {
    const user = userEvent.setup();
    show();
    // each tab of each protocol: the controls of the panel that are not read-only text are stops; the raw editor too
    for (const profile of ["Ufficio Napoli", "Homelab"]) {
      screen.getByRole("option", { name: profile }).focus();
      await user.keyboard("{Enter}");
      for (const tab of screen.getAllByRole("tab")) {
        await user.click(tab);
        const stops = new Set(await tabStops(user));
        const panel = screen.getByRole("tabpanel");
        const controls = [...panel.querySelectorAll<HTMLElement>("input, select, textarea, button, [role='switch']")].filter((el) => el.getAttribute("type") !== "hidden" && el.getAttribute("type") !== "radio" && !(el as HTMLButtonElement).disabled);
        for (const control of controls) expect(stops.has(control), `${profile}, ${tab.textContent}: ${roleOf(control)} «${nameOf(control)}» is a stop`).toBe(true);
      }
    }
    // the menu of the actions: the three entries are reached with the arrows, and an entry that cannot be used stays one of them
    screen.getByRole("button", { name: "More actions" }).focus();
    await user.keyboard("{Enter}");
    const menu = await menuOpen("More actions");
    const entries = within(menu).getAllByRole("menuitem");
    expect(entries.map((e) => nameOf(e))).toEqual(["Duplicate", "Export…", "Delete"].map((n) => expect.stringContaining(n.replace("…", ""))));
    for (const entry of entries) {
      await user.keyboard("{ArrowDown}");
      expect(menu.contains(document.activeElement)).toBe(true);
      expect(entry.isConnected).toBe(true);
    }
    await user.keyboard("{Escape}");
    // «Manage groups»: every control of the dialog is a stop (the focus does not leave the dialog)
    screen.getByRole("button", { name: "Manage groups" }).focus();
    await user.keyboard("{Enter}");
    const dialog = await screen.findByRole("dialog", { name: "Manage groups" });
    const stops = new Set<HTMLElement>([document.activeElement as HTMLElement]);
    for (let step = 0; step < 30; step++) {
      await user.tab();
      // the trap of the dialog sends the focus back from its guards on a later tick, as the browser does
      await new Promise((resolve) => setTimeout(resolve, 20));
      stops.add(document.activeElement as HTMLElement);
    }
    const dialogControls = [...dialog.querySelectorAll<HTMLElement>("input, button")].filter((el) => !(el as HTMLButtonElement).disabled);
    expect(dialogControls.length).toBeGreaterThan(6);
    for (const control of dialogControls) expect(stops.has(control), `the dialog: ${roleOf(control)} «${nameOf(control)}» is a stop`).toBe(true);
    expect([...stops].filter((el) => el !== document.body && !el.hasAttribute("data-base-ui-focus-guard")).every((el) => dialog.contains(el)), `the focus does not leave the dialog: ${[...stops].filter((el) => el !== document.body && !el.hasAttribute("data-base-ui-focus-guard") && !dialog.contains(el)).map((el) => `${roleOf(el)}:${nameOf(el)}`).join(", ")}`).toBe(true);
  });

  it("focus_stays_in_the_page_after_save_duplicate_delete_and_the_groups_dialog", async () => {
    const user = userEvent.setup();
    show();
    const active = () => document.activeElement as HTMLElement;
    const here = (name: string) => expect(`${roleOf(active())}:${nameOf(active())}`, "where the focus is").toBe(name);
    const notBody = (what: string) => expect(active(), `the focus after ${what}`).not.toBe(document.body);

    // a save: the focus stays on the field that was being typed in
    screen.getByRole("textbox", { name: "Name" }).focus();
    await user.keyboard("{End}x{Control>}s{/Control}");
    await waitFor(() => expect(toastStatus().textContent).toContain("Profile saved."));
    here("input:Name");
    // a save that is refused (the name is empty): the focus goes to the field with the error
    await user.keyboard("{Control>}a{/Control}{Backspace}{Control>}s{/Control}");
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Name" }).getAttribute("aria-invalid")).toBe("true"));
    here("input:Name");
    await user.keyboard("Ufficio{Control>}s{/Control}");
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Name" }).getAttribute("aria-invalid")).not.toBe("true"));

    // the dialog of the groups: closed with Escape and closed after a change, the focus is back on its button both times
    const manage = screen.getByRole("button", { name: "Manage groups" });
    for (const closing of ["{Escape}", "{Escape}"]) {
      manage.focus();
      await user.keyboard("{Enter}");
      const dialog = await screen.findByRole("dialog", { name: "Manage groups" });
      await user.type(within(dialog).getByRole("textbox", { name: "New group" }), `G${closing === "{Escape}" ? "x" : "y"}{Enter}`);
      await waitFor(() => expect(within(dialog).getAllByRole("textbox", { name: /^Group \d+$/ }).length).toBeGreaterThan(3));
      await user.keyboard(closing);
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
      expect(active()).toBe(manage);
    }

    // duplicate and export and delete: never the body
    screen.getByRole("button", { name: "More actions" }).focus();
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{ArrowDown}{Enter}"); // Export asks first
    await screen.findByRole("alertdialog", { name: /^Export /});
    notBody("the export asks");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    here("button:More actions");
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{End}{Enter}");
    await screen.findByRole("alertdialog", { name: "Delete Ufficio Napoli?" });
    await user.keyboard("{Enter}"); // «Cancel» has the focus: Enter gives up
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    here("button:More actions");
    await user.keyboard("{Enter}");
    await menuOpen("More actions");
    await user.keyboard("{End}{Enter}");
    await screen.findByRole("alertdialog", { name: "Delete Ufficio Napoli?" });
    await user.tab();
    await user.keyboard("{Enter}"); // «Delete»
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    await waitFor(() => expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(4));
    notBody("the delete");
    expect(roleOf(active())).toBe("option");
  });

  it("no_outcome_depends_on_a_live_region_alone", async () => {
    // every notice of the shell goes through the one `ToastProvider` of `Shell`, and that has `announce`; no file
    // makes a provider of its own without it (Q27, rule 4)
    const sources = (dir: string): string[] =>
      readdirSync(dir).flatMap((name) => {
        const path = join(dir, name);
        return statSync(path).isDirectory() ? sources(path) : /\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name) && !path.includes(`${join("src", "test")}`) && !path.includes(`${join("src", "design")}`) ? [path] : [];
      });
    const providers = sources(SRC).flatMap((file) => [...readFileSync(file, "utf8").matchAll(/<ToastProvider\b[^>]*>/g)].map((m) => ({ file: file.slice(SRC.length), tag: m[0] })));
    expect(providers.map((p) => p.file), "the shell makes its provider").toContain("/shell/Shell.tsx");
    for (const provider of providers) expect(provider.tag, `${provider.file} makes a provider without announce`).toMatch(/announce=\{/);
    // and each outcome of B3 is said, with the text of the notice: save, duplicate, export, delete and a group are
    // polite, an error is assertive
    const user = userEvent.setup();
    const announce = vi.fn();
    let fail = false;
    const update = async (id: string, patch: ProfilePatch) => {
      if (fail) throw { code: "gui_profile_changed", message: "changed", data: null };
      return applyPatch(makeDetail(SAMPLE_PROFILES.find((p) => p.id === id) as ProfileSummary), patch);
    };
    renderShell(makeView(), { announce, update });
    const said = (text: string, assertive: boolean) => waitFor(() => expect(announce).toHaveBeenCalledWith(text, assertive));
    const choose = async (entry: number) => {
      screen.getByRole("button", { name: "More actions" }).focus();
      await user.keyboard("{Enter}");
      await menuOpen("More actions");
      await user.keyboard(`{Home}${"{ArrowDown}".repeat(entry)}{Enter}`);
    };
    screen.getByRole("textbox", { name: "Name" }).focus();
    await user.keyboard("{End}x{Control>}s{/Control}");
    await said("Profile saved.", false);
    await choose(0);
    await said("Duplicated as “Ufficio Napoli (copy)”.", false);
    await choose(1);
    await user.click(await screen.findByRole("button", { name: "Export" }));
    await said("Exported.", false);
    screen.getByRole("button", { name: "Manage groups" }).focus();
    await user.keyboard("{Enter}");
    const dialog = await screen.findByRole("dialog", { name: "Manage groups" });
    await user.type(within(dialog).getByRole("textbox", { name: "New group" }), "Famiglia{Enter}");
    await said("Group Famiglia created.", false);
    await user.keyboard("{Escape}");
    await choose(2);
    await user.keyboard("{Tab}{Enter}");
    await said("Deleted “Ufficio Napoli (copy)”.", false);
    // an error: the text of the notice is the text announced, as assertive
    fail = true;
    screen.getByRole("textbox", { name: "Name" }).focus();
    await user.keyboard("{End}y{Control>}s{/Control}");
    await said("This profile was changed somewhere else since you opened it.", true);
    expect(document.querySelector(".gx-toasts [role=\"alert\"]")?.textContent).toContain("This profile was changed somewhere else since you opened it.");
    // one call for each notice, no more: nothing is said twice
    expect(announce).toHaveBeenCalledTimes(6);
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
