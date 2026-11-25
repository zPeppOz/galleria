import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ToastProvider } from "../design/Toast";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { DEMO_PROFILES } from "./demo-data";
import { ProfileDetail } from "./ProfileDetail";

const [OFFICE, , HOMELAB] = DEMO_PROFILES;
const show = (profile = OFFICE) =>
  render(
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice">
        <main>
          <ProfileDetail profile={profile} groupTitle="Work" />
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
    expect(screen.getAllByText("Work").length).toBeGreaterThan(0); // the meta line, and the option of the group list
    expect(screen.getByText("vpn.ufficio.example:1194")).toBeTruthy();
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

  it("general_tab_has_the_example_form", () => {
    show();
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Ufficio Napoli");
    expect((screen.getByRole("combobox", { name: "Group" }) as HTMLSelectElement).value).toBe("work");
    expect((screen.getByRole("textbox", { name: "Server" }) as HTMLInputElement).value).toBe("vpn.ufficio.example:1194");
    const transport = screen.getByRole("radiogroup", { name: "Transport" });
    expect(transport.getAttribute("aria-readonly")).toBe("true");
    expect(screen.getByRole("switch", { name: "Reconnect automatically" })).toBeTruthy();
  });

  it("wireguard_has_no_transport_choice", () => {
    show(HOMELAB);
    expect(screen.queryByRole("radiogroup", { name: "Transport" })).toBeNull();
  });

  it("more_actions_menu_lists_three_actions", async () => {
    const user = userEvent.setup();
    show();
    const menu = await openMore(user);
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Duplicate", "Export", "Delete"]);
  });

  it.each(["Duplicate", "Export"])("%s_only_shows_the_sample_data_notice", async (action) => {
    const user = userEvent.setup();
    show();
    await openMore(user);
    await user.click(screen.getByRole("menuitem", { name: action }));
    expect(screen.getByRole("status").textContent).toContain("Sample data: nothing was changed");
  });

  it("delete_asks_for_confirmation_and_changes_nothing", async () => {
    const user = userEvent.setup();
    show();
    await openMore(user);
    await user.click(screen.getByRole("menuitem", { name: "Delete" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Delete Ufficio Napoli?" });
    // the safe choice has the focus
    await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Cancel" })));
    await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(screen.getByRole("status").textContent).toBe("");

    await openMore(user);
    await user.click(screen.getByRole("menuitem", { name: "Delete" }));
    await user.click(await screen.findByRole("button", { name: "Delete" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(screen.getByRole("status").textContent).toContain("Sample data: nothing was changed");
  });

  it("focus_does_not_fall_to_the_page_when_the_confirmation_closes", async () => {
    const user = userEvent.setup();
    show();
    await openMore(user);
    await user.click(screen.getByRole("menuitem", { name: "Delete" }));
    await screen.findByRole("alertdialog");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).not.toBe(document.body));
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = show();
    await expectNoAccessibilityViolations(container);
  });
});
