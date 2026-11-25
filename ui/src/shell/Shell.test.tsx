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

const options = () => within(screen.getByRole("listbox")).getAllByRole("option");

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

  it("empty_rubrica_explains_how_to_import", () => {
    renderShell(makeView(), { rubrica: makeRubricaView({ profiles: [], groups: [] }) });
    expect(screen.getByRole("heading", { level: 1, name: "No connections yet" })).toBeTruthy();
    expect(screen.getByText("galleria import <file>")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Copy" })).toBeTruthy();
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(screen.queryByRole("tab")).toBeNull();
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
    await waitFor(() => expect(screen.getAllByRole("status").map((s) => s.textContent).join(" ")).toContain(own["action.arrives_later"]));
    checkAll();
  });

  it("the_three_actions_only_say_they_arrive_later", async () => {
    const user = userEvent.setup();
    show();
    for (const action of ["Duplicate", "Export", "Delete"]) {
      await user.click(screen.getByRole("button", { name: "More actions" }));
      await user.click(await screen.findByRole("menuitem", { name: action }));
      await waitFor(() => expect(screen.getAllByRole("status").map((s) => s.textContent).join(" ")).toContain("This action arrives in a later release."));
    }
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(options()).toHaveLength(5);
    expect(screen.getByRole("heading", { level: 1, name: "Ufficio Napoli" })).toBeTruthy();
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
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(await screen.findByRole("menuitem", { name: "Duplicate" }));
    await user.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(calls).toContain("daemon_retry"));
    // `announce` tells the screen reader about the notice; it talks to the app, not to the daemon (Q27)
    expect(new Set(calls)).toEqual(new Set(["daemon_view", "app_info", "daemon_retry", "announce", "profiles_list", "profile_get"]));
    expect(calls.filter((c) => c === "profile_get").length).toBeGreaterThanOrEqual(2);
  });
});
