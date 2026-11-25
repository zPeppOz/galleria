import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ToastProvider } from "../design/Toast";
import { I18n } from "../i18n";
import type { CmdError, ProfileDetail as Detail, ProfileSummary } from "../rubrica/types";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeDetail, makeSummary, SAMPLE_PROFILES } from "../test/fixtures";
import { ProfileDetail } from "./ProfileDetail";

const [OFFICE, DATACENTER, HOMELAB] = SAMPLE_PROFILES;
const show = (profile: ProfileSummary = OFFICE, detail: Detail | null = makeDetail(profile), detailError: CmdError | null = null) =>
  render(
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice">
        <main>
          <ProfileDetail profile={profile} detail={detail} detailError={detailError} />
        </main>
      </ToastProvider>
    </I18n>,
  );

const openMore = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(screen.getByRole("button", { name: "More actions" }));
  return screen.findByRole("menu", { name: "More actions" });
};

describe("ProfileDetail", () => {
  it("shows_the_name_as_the_level_one_title_with_group_and_server", () => {
    show();
    expect(screen.getByRole("heading", { level: 1, name: "Ufficio Napoli" })).toBeTruthy();
    expect(screen.getAllByText("Lavoro").length).toBeGreaterThan(0);
    expect(screen.getAllByText("vpn.ufficio.example:1194").length).toBeGreaterThan(0);
  });

  it("a_profile_without_a_group_says_so_in_words", () => {
    show(makeSummary({ group: null }));
    expect(screen.getAllByText("No group").length).toBeGreaterThan(0);
  });

  it("a_favourite_has_a_star_with_a_name", () => {
    show();
    expect(screen.getByRole("img", { name: "Favourite" })).toBeTruthy();
  });

  it("protocol_has_a_text_alternative", () => {
    const { unmount } = show(HOMELAB);
    expect(screen.getByText("WG").getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByText("WireGuard")).toBeTruthy();
    unmount();
    show(OFFICE);
    expect(screen.getByText("OVPN").getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByText("OpenVPN")).toBeTruthy();
  });

  it("status_strip_has_four_empty_cells", () => {
    show();
    const cells = screen.getAllByRole("term").map((term) => term.textContent);
    expect(cells).toEqual(["State", "Assigned IP", "Traffic", "Duration"]);
    expect(screen.getAllByText("not available")).toHaveLength(4);
    expect(within(document.body).queryAllByText("—").length).toBe(4);
  });

  it("wireguard_shows_the_last_handshake_instead_of_the_duration", () => {
    show(HOMELAB);
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(["State", "Assigned IP", "Traffic", "Last handshake"]);
  });

  it("tabs_switch_panels", async () => {
    const user = userEvent.setup();
    show();
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["General", "Authentication", "Network", "Configuration", "Log"]);
    expect(screen.getByRole("textbox", { name: "Name" })).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "Log" }));
    expect(screen.getByRole("tabpanel").textContent).toContain("This section arrives in a later release.");
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
  });

  it("general_tab_shows_the_profile_to_read_and_nothing_to_change", () => {
    show();
    const name = screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement;
    expect(name.value).toBe("Ufficio Napoli");
    expect(name.readOnly).toBe(true);
    expect((screen.getByRole("combobox", { name: "Group" }) as HTMLSelectElement).value).toBe("Lavoro");
    const server = screen.getByRole("textbox", { name: "Server" }) as HTMLInputElement;
    expect(server.value).toBe("vpn.ufficio.example:1194");
    expect(server.readOnly).toBe(true);
    const transport = screen.getByRole("radiogroup", { name: "Transport" });
    expect(transport.getAttribute("aria-readonly")).toBe("true");
    expect((within(transport).getByRole("radio", { name: "UDP" }) as HTMLInputElement).checked).toBe(true);
    const reconnect = screen.getByRole("textbox", { name: "Reconnect automatically" }) as HTMLInputElement;
    expect(reconnect.value).toBe("Yes");
    expect(reconnect.readOnly).toBe(true);
    // C makes them switches and fields: until then no control writes anything
    expect(screen.queryByRole("switch")).toBeNull();
  });

  it("the_transport_comes_from_the_profile", () => {
    show(DATACENTER);
    const transport = screen.getByRole("radiogroup", { name: "Transport" });
    expect((within(transport).getByRole("radio", { name: "TCP" }) as HTMLInputElement).checked).toBe(true);
  });

  it("an_automatic_reconnect_that_is_off_says_no", () => {
    const detail = makeDetail(OFFICE);
    show(OFFICE, { ...detail, settings: { ...detail.settings, auto_reconnect: false } });
    expect((screen.getByRole("textbox", { name: "Reconnect automatically" }) as HTMLInputElement).value).toBe("No");
  });

  it("wireguard_has_no_transport_choice", () => {
    show(HOMELAB);
    expect(screen.queryByRole("radiogroup", { name: "Transport" })).toBeNull();
  });

  it("the_form_waits_for_the_detail_and_the_header_does_not", () => {
    show(OFFICE, null);
    expect(screen.getByRole("heading", { level: 1, name: "Ufficio Napoli" })).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
    expect(screen.getByText("Reading the profile…")).toBeTruthy();
  });

  it("a_detail_that_cannot_be_read_shows_the_translated_code", () => {
    show(OFFICE, null, { code: "gui_profile_not_found", message: "Profile not found" });
    expect((document.querySelector(".gx-alert[role=alert]") as HTMLElement).textContent).toContain("This profile no longer exists");
  });

  it("more_actions_menu_lists_three_actions", async () => {
    const user = userEvent.setup();
    show();
    const menu = await openMore(user);
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Duplicate", "Export", "Delete"]);
  });

  it.each(["Duplicate", "Export", "Delete"])("%s_only_says_it_arrives_later", async (action) => {
    const user = userEvent.setup();
    show();
    await openMore(user);
    await user.click(screen.getByRole("menuitem", { name: action }));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("This action arrives in a later release."));
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = show();
    await expectNoAccessibilityViolations(container);
  });
});
