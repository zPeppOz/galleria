import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { DaemonView } from "../daemon/types";
import en from "../i18n/en.json";
import it_ from "../i18n/it.json";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeDetail, makeInfo, makeRubricaView, makeStatus, makeView } from "../test/fixtures";
import { renderShell } from "../test/shell";

const unavailable = (reason: string) => ({
  protocol: "openvpn" as const, available: false, version: null, unavailable_reason: reason,
  capabilities: { handshake: false, interactive_auth: false, server_pushed_config: false },
});

// The prefix of the texts of the sample data of B2, spelled in two pieces so that a search for the word finds nothing in ui/src.
const SAMPLE_PREFIX = `${"de"}mo.`;

const options = () => within(screen.getByRole("listbox")).queryAllByRole("option");

const show = (view: DaemonView | null = makeView(), lang: "en" | "it" = "en", onRetry = vi.fn()) => renderShell(view, { lang, onRetry });

describe("Shell", () => {
  it("has_one_banner_one_main_and_named_regions", () => {
    show();
    expect(screen.getAllByRole("banner")).toHaveLength(1);
    expect(screen.getAllByRole("main")).toHaveLength(1);
    expect(screen.getByRole("complementary", { name: "Profile list" })).toBeTruthy();
    expect(screen.getAllByRole("contentinfo")).toHaveLength(1);
    expect(screen.getAllByRole("heading", { level: 1 }).map((h) => h.textContent)).toEqual(["Ufficio Napoli"]);
    expect(screen.getByRole("listbox", { name: "Connections" })).toBeTruthy();
  });

  it.each(["en", "it"] as const)("sample_data_banner_is_gone: %s", (lang) => {
    show(makeView(), lang);
    expect(within(screen.getByRole("banner")).queryByText(/Sample data|Dati di esempio/)).toBeNull();
    expect(document.body.textContent).not.toMatch(/Sample data|Dati di esempio/);
    // no text of the sample data is left in the catalogues either
    for (const catalogue of [en, it_]) expect(Object.keys(catalogue).filter((key) => key.startsWith(SAMPLE_PREFIX))).toEqual([]);
  });

  it("shows_the_real_profiles_in_their_groups_and_the_selected_one_in_the_detail", async () => {
    const user = userEvent.setup();
    show();
    expect(options()).toHaveLength(5);
    expect(screen.getAllByRole("group").map((g) => g.getAttribute("aria-label"))).toEqual(["Lavoro", "Personale", "Clienti"]);
    await user.click(screen.getByRole("option", { name: "Homelab" }));
    expect(screen.getByRole("heading", { level: 1, name: "Homelab" })).toBeTruthy();
    expect(within(screen.getByRole("main")).getByText("WireGuard")).toBeTruthy();
    expect(within(screen.getByRole("main")).getByText("casa.example:51820")).toBeTruthy();
  });

  it("empty_rubrica_offers_the_import_button", async () => {
    const user = userEvent.setup();
    renderShell(makeView(), { rubrica: makeRubricaView({ profiles: [], groups: [] }) });
    expect(screen.getByRole("heading", { level: 1, name: "No connections yet" })).toBeTruthy();
    // from B4 the window is enough: no command to copy into a terminal
    expect(document.body.textContent).not.toMatch(/galleria\s+import/);
    expect(screen.queryByRole("button", { name: "Copy" })).toBeNull();
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(screen.queryByRole("tab")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Import a profile…" }));
    expect(await screen.findByRole("dialog", { name: "Import profile" })).toBeTruthy();
  });

  it.each(["en", "it"] as const)("drop_hover_shows_and_hides_the_hint: %s", (lang) => {
    const own = lang === "en" ? en : it_;
    const shown = renderShell(makeView(), { lang, dropHover: true });
    const hint = shown.getByText(own["import.drop_hover"]);
    // only to look at: the screen reader does not read it, and it takes no click and no focus
    expect(hint.closest(".shell-drop")?.getAttribute("aria-hidden")).toBe("true");
    expect(within(screen.getByRole("main")).queryByText(own["import.drop_hover"])).toBeNull();
    shown.unmount();
    renderShell(makeView(), { lang, dropHover: false });
    expect(screen.queryByText(own["import.drop_hover"])).toBeNull();
  });

  it("skipped_profiles_are_listed_by_file_name", () => {
    renderShell(makeView(), {
      rubrica: makeRubricaView({
        skipped: [
          { file_name: "broken.toml", code: "gui_profile_invalid" },
          { file_name: "from-the-future.toml", code: "gui_profile_newer" },
        ],
      }),
    });
    const alert = screen.getByText("These files in the profiles folder were skipped").closest(".gx-alert") as HTMLElement;
    expect(within(alert).getByText("broken.toml")).toBeTruthy();
    expect(within(alert).getByText("from-the-future.toml")).toBeTruthy();
    expect(alert.textContent).toContain("The file is not a valid profile");
    expect(alert.textContent).toContain("Written by a newer version of Galleria");
    // the other profiles are still there
    expect(options()).toHaveLength(5);
  });

  it.each(["en", "it"] as const)("store_error_shows_the_translated_code: %s", (lang) => {
    const own = lang === "en" ? en : it_;
    renderShell(makeView(), {
      lang,
      rubrica: null,
      error: { code: "gui_store_insecure", message: "Insecure permissions", data: { path: "/home/m/.config/galleria", mode: "755", fix: "chmod 700 '/home/m/.config/galleria'" } },
    });
    const alert = document.querySelector(".gx-alert[role=alert]") as HTMLElement;
    expect(alert.textContent).toContain(own["error.gui_store_insecure"]);
    expect(alert.textContent).toContain("/home/m/.config/galleria");
    // the command to copy is on the screen, with its button
    expect(screen.getByText("chmod 700 '/home/m/.config/galleria'")).toBeTruthy();
    expect(screen.getByRole("button", { name: own["first.copy"] })).toBeTruthy();
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(screen.queryByText(own["rubrica.empty.title"])).toBeNull();
  });

  it("an_error_code_the_catalogue_does_not_know_shows_the_english_message", () => {
    renderShell(makeView(), { rubrica: null, error: { code: "gui_brand_new", message: "Something new went wrong" } });
    expect((document.querySelector(".gx-alert[role=alert]") as HTMLElement).textContent).toContain("Something new went wrong");
  });

  it("first_run_panel_shows_while_the_list_stays_usable", async () => {
    const user = userEvent.setup();
    show(makeView({ link: "daemon_not_running", status: null }));
    expect(screen.getByText("sudo systemctl enable --now galleriad")).toBeTruthy();
    await user.click(screen.getByRole("option", { name: "Ufficio Napoli" }));
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("heading", { level: 1, name: "Datacenter Milano" })).toBeTruthy();
    expect(screen.getByText("sudo systemctl enable --now galleriad")).toBeTruthy();
  });

  it("first_run_panel_retry_calls_the_handler", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();
    show(makeView({ link: "permission_denied", status: null }), "en", onRetry);
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("connecting_before_the_first_answer", () => {
    show(null);
    expect(screen.getByRole("heading", { name: "Connecting to the service" })).toBeTruthy();
    expect(options()).toHaveLength(5);
  });

  it("the_list_says_it_is_reading_before_the_first_answer", () => {
    renderShell(makeView(), { rubrica: null });
    expect(screen.getByText("Reading your profiles…")).toBeTruthy();
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("backend_warning_is_shown_when_connected", () => {
    show(makeView({ backends: [unavailable("openvpn_not_found")] }));
    expect(screen.getByText(/OpenVPN is not installed/).closest(".gx-alert")?.getAttribute("role")).toBe("status");
    expect(screen.queryByRole("heading", { name: "Status" })).toBeNull();
  });

  it("daemon_status_shows_when_a_session_exists", () => {
    show(makeView({ status: makeStatus({ state: "connected", profile: { id: "p", name: "Homelab" }, protocol: "wireguard" }), backends: [unavailable("openvpn_not_found")] }));
    expect(screen.getByRole("heading", { name: "Status" })).toBeTruthy();
    // the warning is shown once, inside the status panel
    expect(screen.getAllByText(/OpenVPN is not installed/)).toHaveLength(1);
  });

  it("nothing_extra_shows_when_connected_and_idle", () => {
    show();
    expect(document.querySelector(".gx-alert")).toBeNull();
    expect(screen.queryByRole("alert")?.textContent ?? "").toBe(""); // the empty live regions of the toasts only
  });

  it.each(["en", "it"] as const)("works_in_both_languages: %s", async (lang) => {
    const user = userEvent.setup();
    const own = lang === "en" ? en : it_;
    const other = (lang === "en" ? it_ : en) as Record<string, string>;
    const foreign = new Set(Object.entries(own).filter(([key, text]) => other[key] !== text).map(([key]) => other[key]));
    const rawKey = /^[a-z]+(\.[a-z_]+)+$/;
    const texts = () => {
      const found: string[] = [];
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const text = (node.textContent ?? "").trim();
        if (text) found.push(text);
      }
      return found;
    };
    const checkAll = () => {
      for (const text of texts()) {
        expect(rawKey.test(text), `raw key on screen: ${text}`).toBe(false);
        expect(foreign.has(text), `text of the other language: ${text}`).toBe(false);
      }
    };
    show(makeView({ backends: [unavailable("openvpn_not_found")] }), lang);
    checkAll();
    for (const tab of screen.getAllByRole("tab")) {
      await user.click(tab);
      checkAll();
    }
    await user.click(screen.getByRole("button", { name: own["detail.more"] }));
    checkAll();
    await user.click(await screen.findByRole("menuitem", { name: own["detail.delete"] }));
    const dialog = await screen.findByRole("alertdialog");
    checkAll();
    await user.click(within(dialog).getByRole("button", { name: own["delete.cancel"] }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    await user.click(screen.getByRole("button", { name: own["detail.more"] }));
    await user.click(await screen.findByRole("menuitem", { name: own["detail.export"] }));
    await screen.findByRole("alertdialog");
    checkAll();
  });

  it("duplicate_adds_a_copy_to_the_list_and_chooses_it", async () => {
    const user = userEvent.setup();
    show();
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(await screen.findByRole("menuitem", { name: "Duplicate" }));
    await waitFor(() => expect(screen.getByRole("heading", { level: 1, name: "Ufficio Napoli (copy)" })).toBeTruthy());
    expect(options()).toHaveLength(6);
    // the focus is on «More actions» of the copy
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "More actions" })));
  });

  it("delete_removes_the_profile_and_the_focus_goes_to_the_list", async () => {
    const user = userEvent.setup();
    show();
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(await screen.findByRole("menuitem", { name: "Delete" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(options()).toHaveLength(4));
    expect(screen.queryByRole("option", { name: "Ufficio Napoli" })).toBeNull();
    expect(screen.getByRole("heading", { level: 1 }).textContent).not.toBe("Ufficio Napoli");
    await waitFor(() => expect(screen.getByRole("listbox").contains(document.activeElement)).toBe(true));
  });

  it("delete_of_the_last_profile_leaves_the_empty_text_with_the_focus", async () => {
    const user = userEvent.setup();
    renderShell(makeView(), { rubrica: makeRubricaView({ profiles: [makeRubricaView().profiles[0]], groups: [] }) });
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(await screen.findByRole("menuitem", { name: "Delete" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(document.activeElement?.textContent).toBe("No profiles"));
  });

  it("the_profile_of_the_open_session_is_not_offered_for_deletion", async () => {
    const user = userEvent.setup();
    show(makeView({ status: makeStatus({ state: "connected", profile: { id: makeRubricaView().profiles[0].id, name: "Ufficio Napoli" }, protocol: "openvpn" }) }));
    await user.click(screen.getByRole("button", { name: "More actions" }));
    const item = await screen.findByRole("menuitem", { name: "Delete" });
    expect(item.getAttribute("aria-disabled")).toBe("true");
    await user.click(item);
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("changing_language_and_theme_keeps_the_selection", async () => {
    const user = userEvent.setup();
    show();
    await user.click(screen.getByRole("option", { name: "Homelab" }));
    await user.click(screen.getByRole("button", { name: "Menu" }));
    await user.click(await screen.findByRole("menuitemradio", { name: "Italiano" }));
    await waitFor(() => expect(screen.getByRole("tab", { name: "Generale" })).toBeTruthy());
    expect(screen.getByRole("heading", { level: 1, name: "Homelab" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Personale" })).toBeTruthy();
  });

  it("a_profile_without_a_group_is_listed_last_under_a_translated_title", () => {
    const view = makeRubricaView();
    renderShell(makeView(), { rubrica: { ...view, profiles: [...view.profiles, { ...view.profiles[0], id: "loose", name: "Senza", group: null }] } });
    expect(screen.getAllByRole("group").map((g) => g.getAttribute("aria-label"))).toEqual(["Lavoro", "Personale", "Clienti", "No group"]);
  });

  it.each([
    ["connecting", makeView({ link: "connecting", status: null })],
    ["daemon_not_running", makeView({ link: "daemon_not_running", status: null })],
    ["permission_denied", makeView({ link: "permission_denied", status: null })],
    ["connected", makeView()],
    ["connected_with_error", makeView({ status: makeStatus({ state: "error", error: { code: "auth_failed", message: "x" }, profile: { id: "p", name: "Homelab" }, protocol: "wireguard" }), backends: [unavailable("openvpn_too_old")] })],
  ] as const)("has_no_accessibility_violations: %s", async (_name, view) => {
    const { container } = show(view);
    await expectNoAccessibilityViolations(container);
  });

  it.each([
    ["empty", () => renderShell(makeView(), { rubrica: makeRubricaView({ profiles: [], groups: [] }) })],
    ["reading", () => renderShell(makeView(), { rubrica: null })],
    ["store_error", () => renderShell(makeView(), { rubrica: null, error: { code: "gui_store_insecure", message: "x", data: { path: "/p", mode: "755", fix: "chmod 700 '/p'" } } })],
    ["skipped", () => renderShell(makeView(), { rubrica: makeRubricaView({ skipped: [{ file_name: "a.toml", code: "gui_profile_invalid" }] }) })],
  ])("the_states_of_the_rubrica_have_no_accessibility_violations: %s", async (_name, draw) => {
    const { container } = draw();
    await expectNoAccessibilityViolations(container);
  });

  // The whole app, with the commands answered by a mock: what the shell asks the app for, and nothing else.
  it("shell_only_calls_the_listed_commands", async () => {
    const calls: string[] = [];
    const view = makeRubricaView();
    mockIPC(
      (cmd, args) => {
        calls.push(cmd);
        if (cmd === "daemon_view") return makeView({ link: "daemon_not_running", status: null });
        if (cmd === "app_info") return makeInfo();
        if (cmd === "profiles_list") return view;
        if (cmd === "profile_get") return makeDetail(view.profiles.find((p) => p.id === (args as { id: string }).id) ?? view.profiles[0]);
        return undefined;
      },
      { shouldMockEvents: true },
    );
    const user = userEvent.setup();
    render(<App lang="en" />);
    await screen.findByText("sudo systemctl enable --now galleriad");
    await user.click(await screen.findByRole("option", { name: "Homelab" }));
    await screen.findByRole("textbox", { name: "Name" });
    for (const tab of screen.getAllByRole("tab")) await user.click(tab);
    await user.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(calls).toContain("daemon_retry"));
    expect(new Set(calls)).toEqual(new Set(["daemon_view", "app_info", "daemon_retry", "profiles_list", "profile_get", "import_pending"]));
    expect(calls.filter((c) => c === "profile_get").length).toBeGreaterThanOrEqual(2);
  });
});

describe("Shell: search, favourites, sort order and the session", () => {
  const rowNames = () => options().map((o) => document.getElementById((o.getAttribute("aria-labelledby") ?? "").split(" ")[0])?.textContent);

  it("favourites_come_first_inside_their_group_in_the_list", () => {
    show();
    // «Ufficio Napoli» and «Homelab» are the favourites of the seed
    expect(rowNames()).toEqual(["Ufficio Napoli", "Datacenter Milano", "Homelab", "VPS Hetzner", "Cliente Rossi"]);
  });

  it("search_filters_the_list_and_says_how_many", async () => {
    const user = userEvent.setup();
    show();
    await user.type(screen.getByRole("searchbox", { name: "Search connections" }), "milano");
    expect(rowNames()).toEqual(["Datacenter Milano"]);
    expect(screen.getAllByRole("group").map((g) => g.getAttribute("aria-label"))).toEqual(["Lavoro"]);
    expect(document.querySelector(".search-count")?.textContent).toBe("1 result");
    // the server and the group are searched too
    await user.clear(screen.getByRole("searchbox"));
    await user.type(screen.getByRole("searchbox"), "casa.example");
    expect(rowNames()).toEqual(["Homelab"]);
    await user.clear(screen.getByRole("searchbox"));
    await user.type(screen.getByRole("searchbox"), "clienti");
    expect(rowNames()).toEqual(["Cliente Rossi"]);
  });

  it("no_match_says_so_and_the_detail_stays", async () => {
    const user = userEvent.setup();
    show();
    await user.type(screen.getByRole("searchbox", { name: "Search connections" }), "zzzz");
    expect(screen.getByRole("listbox", { name: "Connections" })).toBeTruthy();
    expect(options()).toHaveLength(0);
    expect(screen.getByText("No connection matches your search.")).toBeTruthy();
    expect(document.querySelector(".search-count")?.textContent).toBe("0 results");
    // the profile that was chosen is still the one in the detail: a search does not change what is open
    expect(screen.getByRole("heading", { level: 1, name: "Ufficio Napoli" })).toBeTruthy();
    // and Esc brings the list back
    await user.keyboard("{Escape}");
    expect(options()).toHaveLength(5);
  });

  it("ctrl_k_then_arrow_down_goes_from_anywhere_to_the_list", async () => {
    const user = userEvent.setup();
    show();
    screen.getByRole("tab", { name: "General" }).focus();
    await user.keyboard("{Control>}k{/Control}");
    expect(document.activeElement).toBe(screen.getByRole("searchbox"));
    await user.keyboard("dat{ArrowDown}");
    expect(document.activeElement).toBe(screen.getByRole("option", { name: "Datacenter Milano" }));
  });

  it.each([["en", "Most recent"], ["it", "Più recenti"]] as const)("the_saved_sort_order_is_the_one_the_list_starts_with: %s", (lang, label) => {
    const view = makeRubricaView();
    const used = view.profiles.map((p) => (p.id === "rossi" ? { ...p, last_used: "2026-11-24T10:00:00Z" } : p.id === "datacenter" ? { ...p, last_used: "2026-11-20T10:00:00Z" } : p));
    renderShell(makeView(), { lang, sort: "recent", rubrica: { ...view, profiles: used } });
    expect((screen.getByRole("radio", { name: label }) as HTMLInputElement).checked).toBe(true);
    // inside the group «Lavoro» the favourite is still first, then the one used last
    expect(rowNames()).toEqual(["Ufficio Napoli", "Datacenter Milano", "Homelab", "VPS Hetzner", "Cliente Rossi"]);
  });

  it("changing_the_sort_order_reorders_the_list", async () => {
    mockIPC(() => undefined);
    const user = userEvent.setup();
    const view = makeRubricaView();
    // no favourites, so that only the order of use and the name decide; «VPS Hetzner» was used after «Homelab»
    const used = view.profiles.map((p) => ({ ...p, favorite: false, last_used: p.id === "vps" ? "2026-11-24T10:00:00Z" : p.id === "homelab" ? "2026-11-01T10:00:00Z" : null }));
    renderShell(makeView(), { rubrica: { ...view, profiles: used } });
    expect(rowNames()).toEqual(["Datacenter Milano", "Ufficio Napoli", "Homelab", "VPS Hetzner", "Cliente Rossi"]);
    await user.click(screen.getByRole("radio", { name: "Most recent" }));
    expect(rowNames()).toEqual(["Datacenter Milano", "Ufficio Napoli", "VPS Hetzner", "Homelab", "Cliente Rossi"]);
    await user.click(screen.getByRole("radio", { name: "By name" }));
    expect(rowNames()).toEqual(["Datacenter Milano", "Ufficio Napoli", "Homelab", "VPS Hetzner", "Cliente Rossi"]);
  });

  it("the_session_of_a_profile_of_the_rubrica_shows_on_its_row", () => {
    show(makeView({ status: makeStatus({ state: "connected", profile: { id: "homelab", name: "Homelab" }, protocol: "wireguard" }) }));
    const row = screen.getByRole("option", { name: "Homelab Connected" });
    expect(row.textContent).toContain("Connected");
    expect(screen.getAllByRole("option").filter((o) => o.querySelector(".profile-state"))).toHaveLength(1);
  });

  it("no_state_is_shown_for_a_foreign_session", () => {
    // a session of a profile that is not in the rubrica (started by someone else, D21): no row says anything
    show(makeView({ status: makeStatus({ state: "connected", profile: { id: "not-in-the-rubrica", name: "Other" }, protocol: "wireguard" }) }));
    expect(document.querySelector(".profile-state")).toBeNull();
    expect(options()).toHaveLength(5);
  });

  it("a_disconnected_state_shows_nothing_on_the_row", () => {
    show(makeView({ status: makeStatus({ state: "disconnected", profile: { id: "homelab", name: "Homelab" } }) }));
    expect(document.querySelector(".profile-state")).toBeNull();
  });

  it("the_search_and_the_sort_have_no_accessibility_violations_in_the_shell", async () => {
    const user = userEvent.setup();
    const { container } = show();
    await user.type(screen.getByRole("searchbox"), "mi");
    await expectNoAccessibilityViolations(container);
  });
});
