import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { DaemonView } from "../daemon/types";
import en from "../i18n/en.json";
import it_ from "../i18n/it.json";
import { Preferences } from "../prefs";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeInfo, makeStatus, makeView } from "../test/fixtures";
import { Shell } from "./Shell";

const unavailable = (reason: string) => ({
  protocol: "openvpn" as const, available: false, version: null, unavailable_reason: reason,
  capabilities: { handshake: false, interactive_auth: false, server_pushed_config: false },
});

const options = () => within(screen.getByRole("listbox")).getAllByRole("option");

const show = (view: DaemonView | null = makeView(), lang: "en" | "it" = "en", onRetry = vi.fn()) =>
  render(
    <Preferences lang={lang}>
      <Shell view={view} info={view ? makeInfo() : null} onRetry={onRetry} />
    </Preferences>,
  );

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

  it.each([["en", "Sample data"], ["it", "Dati di esempio"]] as const)("demo_banner_is_visible: %s", (lang, text) => {
    show(makeView(), lang);
    expect(within(screen.getByRole("banner")).getByText(text)).toBeTruthy();
  });

  it("shows_the_five_sample_profiles_and_the_selected_one_in_the_detail", async () => {
    const user = userEvent.setup();
    show();
    expect(options()).toHaveLength(5);
    await user.click(screen.getByRole("option", { name: "Homelab" }));
    expect(screen.getByRole("heading", { level: 1, name: "Homelab" })).toBeTruthy();
    expect(within(screen.getByRole("main")).getByText("WireGuard")).toBeTruthy();
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
    await screen.findByRole("alertdialog");
    checkAll();
  });

  it("delete_changes_nothing_in_the_list", async () => {
    const user = userEvent.setup();
    show();
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(await screen.findByRole("menuitem", { name: "Delete" }));
    await user.click(await screen.findByRole("button", { name: "Delete" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(screen.getByRole("status").textContent).toContain("Sample data: nothing was changed");
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

  it("shell_never_calls_the_daemon", async () => {
    const calls: string[] = [];
    mockIPC(
      (cmd) => {
        calls.push(cmd);
        return cmd === "daemon_view" ? makeView({ link: "daemon_not_running", status: null }) : cmd === "app_info" ? makeInfo() : undefined;
      },
      { shouldMockEvents: true },
    );
    const user = userEvent.setup();
    render(<App lang="en" />);
    await screen.findByText("sudo systemctl enable --now galleriad");
    // a full round of actions: selection, tabs, the menus, delete with confirmation, the retry button
    await user.click(screen.getByRole("option", { name: "Homelab" }));
    for (const tab of screen.getAllByRole("tab")) await user.click(tab);
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(await screen.findByRole("menuitem", { name: "Duplicate" }));
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(await screen.findByRole("menuitem", { name: "Delete" }));
    await user.click(await screen.findByRole("button", { name: "Delete" }));
    await user.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(calls).toContain("daemon_retry"));
    // `announce` tells the screen reader about the two notices of the round; it talks to the app, not to the daemon (Q27)
    expect(new Set(calls)).toEqual(new Set(["daemon_view", "app_info", "daemon_retry", "announce"]));
  });
});
