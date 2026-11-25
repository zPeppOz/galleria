import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../design/Toast";
import { I18n } from "../i18n";
import type { CmdError, ProfileDetail as Detail, ProfilePatch, ProfileSummary } from "../rubrica/types";
import { expectNoAccessibilityViolations } from "../test/axe";
import { applyPatch, makeDetail, makeSummary, SAMPLE_PROFILES } from "../test/fixtures";
import { renderShell } from "../test/shell";
import { withGoodKey } from "../test/tabs";
import { ProfileDetail } from "./ProfileDetail";

const [OFFICE, DATACENTER, HOMELAB] = SAMPLE_PROFILES;

type Update = (id: string, patch: ProfilePatch, expectedModified: string) => Promise<Detail>;

/** The form with a backend that answers what a test says (by default: the patch applied) and the calls it received. */
const setup = (profile: ProfileSummary = OFFICE, detail: Detail | null = makeDetail(profile), detailError: CmdError | null = null, update?: Update) => {
  const send = vi.fn<Update>(update ?? (async (_id, patch) => applyPatch(detail ?? makeDetail(profile), patch)));
  const reload = vi.fn(async () => undefined);
  const announce = vi.fn();
  const tree = (current: Detail | null) => (
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice" announce={announce}>
        <main>
          <ProfileDetail profile={profile} detail={current} detailError={detailError} groups={["Lavoro", "Personale", "Clienti"]} update={send} reload={reload} />
        </main>
      </ToastProvider>
    </I18n>
  );
  const view = render(tree(detail));
  return { ...view, send, reload, announce, show: (next: Detail | null) => view.rerender(tree(next)) };
};

const toasts = () => document.querySelector<HTMLElement>(".gx-toasts [role=status]")!;
const alerts = () => document.querySelector<HTMLElement>(".gx-toasts [role=alert]")!;
const typeInto = async (user: ReturnType<typeof userEvent.setup>, name: string, text: string) => {
  const field = screen.getByRole("textbox", { name });
  await user.clear(field);
  if (text) await user.type(field, text);
  return field;
};

const openMore = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(screen.getByRole("button", { name: "More actions" }));
  return screen.findByRole("menu", { name: "More actions" });
};

describe("ProfileDetail", () => {
  it("shows_the_name_as_the_level_one_title_with_group_and_server", () => {
    setup();
    expect(screen.getByRole("heading", { level: 1, name: "Ufficio Napoli" })).toBeTruthy();
    expect(screen.getAllByText("Lavoro").length).toBeGreaterThan(0);
    expect(screen.getAllByText("vpn.ufficio.example:1194").length).toBeGreaterThan(0);
  });

  it("a_profile_without_a_group_says_so_in_words", () => {
    setup(makeSummary({ group: null }));
    expect(screen.getAllByText("No group").length).toBeGreaterThan(0);
  });

  it("the_star_is_a_toggle_with_a_name_and_a_state", () => {
    const { unmount } = setup();
    expect(screen.getByRole("button", { name: "Favourite", pressed: true })).toBeTruthy();
    unmount();
    setup(DATACENTER);
    expect(screen.getByRole("button", { name: "Favourite", pressed: false })).toBeTruthy();
  });

  it("while_the_detail_is_not_there_the_star_is_only_a_picture_with_a_name", () => {
    setup(OFFICE, null);
    expect(screen.getByRole("img", { name: "Favourite" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Favourite" })).toBeNull();
  });

  it("protocol_has_a_text_alternative", () => {
    const { unmount } = setup(HOMELAB);
    expect(screen.getByText("WG").getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByText("WireGuard")).toBeTruthy();
    unmount();
    setup(OFFICE);
    expect(screen.getByText("OVPN").getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByText("OpenVPN")).toBeTruthy();
  });

  it("status_strip_has_four_empty_cells", () => {
    setup();
    const cells = screen.getAllByRole("term").map((term) => term.textContent);
    expect(cells).toEqual(["State", "Assigned IP", "Traffic", "Duration"]);
    expect(screen.getAllByText("not available")).toHaveLength(4);
    expect(within(document.body).queryAllByText("—").length).toBe(4);
  });

  it("wireguard_shows_the_last_handshake_instead_of_the_duration", () => {
    setup(HOMELAB);
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(["State", "Assigned IP", "Traffic", "Last handshake"]);
  });

  it("tabs_switch_panels", async () => {
    const user = userEvent.setup();
    setup();
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["General", "Authentication", "Network", "Configuration", "Log"]);
    expect(screen.getByRole("textbox", { name: "Name" })).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "Authentication" }));
    expect(screen.getByRole("heading", { name: "Method" })).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "Network" }));
    expect(screen.getByRole("switch", { name: "Block IPv6" })).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "Log" }));
    expect(screen.getByRole("tabpanel").textContent).toContain("This section arrives in a later release.");
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
  });

  it("the_form_waits_for_the_detail_and_the_header_does_not", () => {
    setup(OFFICE, null);
    expect(screen.getByRole("heading", { level: 1, name: "Ufficio Napoli" })).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
    expect(screen.getByText("Reading the profile…")).toBeTruthy();
  });

  it("a_detail_that_cannot_be_read_shows_the_translated_code", () => {
    setup(OFFICE, null, { code: "gui_profile_not_found", message: "Profile not found" });
    expect((document.querySelector(".gx-alert[role=alert]") as HTMLElement).textContent).toContain("This profile no longer exists");
  });

  it("more_actions_menu_lists_three_actions", async () => {
    const user = userEvent.setup();
    setup();
    const menu = await openMore(user);
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Duplicate", "Export", "Delete"]);
  });

  it.each(["Duplicate", "Export", "Delete"])("%s_only_says_it_arrives_later", async (action) => {
    const user = userEvent.setup();
    setup();
    await openMore(user);
    await user.click(screen.getByRole("menuitem", { name: action }));
    await waitFor(() => expect(toasts().textContent).toContain("This action arrives in a later release."));
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = setup();
    await expectNoAccessibilityViolations(container);
  });
});

describe("ProfileDetail, editing", () => {
  it("the_save_bar_is_there_only_while_something_is_changed_and_cancel_puts_the_values_back", async () => {
    const user = userEvent.setup();
    setup();
    expect(screen.queryByRole("group", { name: "Unsaved changes" })).toBeNull();
    const name = await typeInto(user, "Name", "Sede");
    expect(screen.getByRole("group", { name: "Unsaved changes" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect((name as HTMLInputElement).value).toBe("Ufficio Napoli");
    expect(screen.queryByRole("group", { name: "Unsaved changes" })).toBeNull();
  });

  it("save_shows_a_notice_and_keeps_the_focus", async () => {
    const user = userEvent.setup();
    const { send, reload, announce } = setup();
    const name = await typeInto(user, "Name", "  Sede  ");
    expect(document.activeElement).toBe(name);
    await user.keyboard("{Control>}s{/Control}");
    await waitFor(() => expect(toasts().textContent).toContain("Profile saved."));
    // the patch has only what changed, the name trimmed, and the time of modification the form was built from
    expect(send).toHaveBeenCalledWith("office", { name: "Sede" }, OFFICE.modified);
    expect(reload).toHaveBeenCalled();
    expect(document.activeElement).toBe(name);
    expect(announce).toHaveBeenCalledWith("Profile saved.", false);
    expect(screen.queryByRole("group", { name: "Unsaved changes" })).toBeNull();
    // the header follows what was saved, and what the form shows is what is on the disk now
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Sede");
    expect((name as HTMLInputElement).value).toBe("Sede");
  });

  it("the_save_button_gives_the_focus_to_the_selected_tab_when_the_bar_goes_away", async () => {
    const user = userEvent.setup();
    setup();
    await typeInto(user, "Name", "Sede");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(toasts().textContent).toContain("Profile saved."));
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "General" }));
  });

  it("a_validation_error_moves_the_focus_to_the_first_bad_field_and_shows_no_notice", async () => {
    const user = userEvent.setup();
    const { send, announce } = setup(HOMELAB, withGoodKey(makeDetail(HOMELAB)));
    await user.click(screen.getByRole("tab", { name: "Network" }));
    await typeInto(user, "MTU", "100");
    await user.click(screen.getByRole("button", { name: "Save" }));
    const mtu = screen.getByRole("textbox", { name: "MTU" });
    await waitFor(() => expect(document.activeElement).toBe(mtu));
    expect(mtu.getAttribute("aria-invalid")).toBe("true");
    // the error is read with the field, and nothing else announces it (Q27, rule 2)
    const described = (mtu.getAttribute("aria-describedby") ?? "").split(" ").map((id) => document.getElementById(id)?.textContent);
    expect(described).toContain("Use a number from 576 to 9000.");
    expect(send).not.toHaveBeenCalled();
    expect(toasts().textContent).toBe("");
    expect(alerts().textContent).toBe("");
    expect(announce).not.toHaveBeenCalled();
    // typing in the field takes its error away
    await user.type(mtu, "0");
    expect(mtu.getAttribute("aria-invalid")).toBeNull();
  });

  it("the_first_bad_field_is_the_first_in_the_order_of_the_tabs_and_its_tab_opens", async () => {
    const user = userEvent.setup();
    setup(HOMELAB, withGoodKey(makeDetail(HOMELAB)));
    await typeInto(user, "Name", "");
    await user.click(screen.getByRole("tab", { name: "Network" }));
    await typeInto(user, "MTU", "100");
    await user.click(screen.getByRole("button", { name: "Save" }));
    // the name (General) comes before the MTU (Network): the form goes back to General and puts the focus there
    const name = await screen.findByRole("textbox", { name: "Name" });
    await waitFor(() => expect(document.activeElement).toBe(name));
    expect(name.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById(name.getAttribute("aria-describedby") ?? "")?.textContent).toBe("This field is required.");
  });

  it("a_refusal_of_the_backend_for_a_field_is_handled_the_same_way", async () => {
    const user = userEvent.setup();
    const refusal: CmdError = { code: "gui_invalid_field", message: "Some fields are not valid", data: { fields: [{ field: "peers.0.endpoint", code: "invalid_endpoint" }] } };
    const { announce } = setup(HOMELAB, withGoodKey(makeDetail(HOMELAB)), null, () => Promise.reject(refusal));
    await typeInto(user, "Endpoint", "altro.example:51820");
    await user.click(screen.getByRole("button", { name: "Save" }));
    const endpoint = screen.getByRole("textbox", { name: "Endpoint" });
    await waitFor(() => expect(document.activeElement).toBe(endpoint));
    expect(document.getElementById(endpoint.getAttribute("aria-describedby") ?? "")?.textContent).toBe("Use host:port, like vpn.example.com:51820.");
    expect(announce).not.toHaveBeenCalled();
    // and what was typed is still there
    expect((endpoint as HTMLInputElement).value).toBe("altro.example:51820");
  });

  it("a_refusal_that_is_not_about_a_field_is_an_error_notice_and_the_draft_stays", async () => {
    const user = userEvent.setup();
    const { announce } = setup(OFFICE, makeDetail(OFFICE), null, () => Promise.reject({ code: "gui_store_unreadable", message: "x" }));
    await typeInto(user, "Name", "Sede");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(alerts().textContent).toContain("The profiles cannot be read"));
    expect(announce).toHaveBeenCalledWith("The profiles cannot be read", true);
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Sede");
    expect(screen.getByRole("group", { name: "Unsaved changes" })).toBeTruthy();
  });

  it("switches_write_at_once_and_revert_on_failure", async () => {
    const user = userEvent.setup();
    let fail!: (error: CmdError) => void;
    const { send, reload, announce } = setup(OFFICE, makeDetail(OFFICE), null, () => new Promise((_resolve, reject) => { fail = reject; }));
    const toggle = screen.getByRole("switch", { name: "Reconnect automatically" });
    await user.click(toggle);
    // it already shows the new value while the write goes, and there is no save bar for it
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(screen.queryByRole("group", { name: "Unsaved changes" })).toBeNull();
    expect(send).toHaveBeenCalledWith("office", { settings: { ipv6_block: true, auto_reconnect: false, ignore_pushed_routes: false } }, OFFICE.modified);
    await act(async () => fail({ code: "gui_store_insecure", message: "x" }));
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    await waitFor(() => expect(alerts().textContent).toContain("open to other users"));
    expect(announce).toHaveBeenCalledWith(expect.stringContaining("open to other users"), true);
    expect(reload).not.toHaveBeenCalled();
  });

  it("a_switch_that_works_says_nothing_and_the_list_is_read_again", async () => {
    const user = userEvent.setup();
    const { reload, announce } = setup();
    await user.click(screen.getByRole("tab", { name: "Network" }));
    await user.click(screen.getByRole("switch", { name: "Ignore the routes the server pushes" }));
    await waitFor(() => expect(reload).toHaveBeenCalled());
    expect(screen.getByRole("switch", { name: "Ignore the routes the server pushes" }).getAttribute("aria-checked")).toBe("true");
    expect(toasts().textContent).toBe("");
    expect(announce).not.toHaveBeenCalled();
  });

  it("the_star_writes_at_once_too", async () => {
    const user = userEvent.setup();
    const { send } = setup();
    await user.click(screen.getByRole("button", { name: "Favourite" }));
    expect(send).toHaveBeenCalledWith("office", { favorite: false }, OFFICE.modified);
    await waitFor(() => expect(screen.getByRole("button", { name: "Favourite" }).getAttribute("aria-pressed")).toBe("false"));
  });

  it("a_switch_does_not_throw_away_what_is_typed", async () => {
    const user = userEvent.setup();
    setup();
    await typeInto(user, "Name", "Sede");
    await user.click(screen.getByRole("switch", { name: "Reconnect automatically" }));
    await waitFor(() => expect(screen.getByRole("switch", { name: "Reconnect automatically" }).getAttribute("aria-checked")).toBe("false"));
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Sede");
    expect(screen.getByRole("group", { name: "Unsaved changes" })).toBeTruthy();
  });

  it("a_changed_profile_offers_reload_and_keeps_the_draft", async () => {
    const user = userEvent.setup();
    const { show, reload } = setup();
    await typeInto(user, "Name", "Sede");
    // the CLI wrote the profile: the same one, with another time and another name
    const elsewhere = { ...makeDetail(OFFICE), name: "Dalla CLI", modified: "2026-11-25T11:00:00Z" };
    show(elsewhere);
    const alert = await screen.findByText(/changed somewhere else, for example from the command line/);
    expect(alert).toBeTruthy();
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Sede");
    await user.click(screen.getByRole("button", { name: "Reload" }));
    expect(reload).toHaveBeenCalled();
    await waitFor(() => expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Dalla CLI"));
    expect(screen.queryByText(/changed somewhere else/)).toBeNull();
    expect(screen.queryByRole("group", { name: "Unsaved changes" })).toBeNull();
  });

  it("a_profile_that_changed_while_nothing_was_edited_is_taken_in_without_asking", () => {
    const { show } = setup();
    show({ ...makeDetail(OFFICE), name: "Dalla CLI", modified: "2026-11-25T11:00:00Z" });
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Dalla CLI");
    expect(screen.queryByText(/changed somewhere else/)).toBeNull();
  });

  it("a_save_that_finds_the_profile_changed_says_so_keeps_the_draft_and_announces_it_as_an_error", async () => {
    const user = userEvent.setup();
    const { announce, reload } = setup(OFFICE, makeDetail(OFFICE), null, () => Promise.reject({ code: "gui_profile_changed", message: "x" }));
    await typeInto(user, "Name", "Sede");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(alerts().textContent).toContain("changed somewhere else since you opened it"));
    expect(announce).toHaveBeenCalledWith(expect.stringContaining("changed somewhere else since you opened it"), true);
    expect(screen.getByRole("button", { name: "Reload" })).toBeTruthy();
    expect(reload).toHaveBeenCalled();
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Sede");
  });

  it("leaving_with_unsaved_changes_asks_first", async () => {
    const user = userEvent.setup();
    renderShell();
    await typeInto(user, "Name", "Sede");
    await user.click(screen.getByRole("option", { name: /Homelab/ }));
    const dialog = await screen.findByRole("alertdialog", { name: "Save your changes?" });
    expect(dialog.textContent).toContain("Ufficio Napoli has changes that are not saved.");
    expect(within(dialog).getAllByRole("button").map((b) => b.textContent)).toEqual(["Cancel", "Discard", "Save"]);
    // Cancel: nothing moves and nothing is lost
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Ufficio Napoli");
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Sede");
    // Discard: the other profile opens
    await user.click(screen.getByRole("option", { name: /Homelab/ }));
    await user.click(await screen.findByRole("button", { name: "Discard" }));
    await waitFor(() => expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Homelab"));
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("leaving_with_save_writes_first_and_moves_when_it_worked", async () => {
    const user = userEvent.setup();
    const update = vi.fn<Update>(async (_id, patch) => applyPatch(makeDetail(OFFICE), patch));
    renderShell(undefined, { update });
    await typeInto(user, "Name", "Sede");
    await user.click(screen.getByRole("option", { name: /Homelab/ }));
    await user.click(await screen.findByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Homelab"));
    expect(update).toHaveBeenCalledWith("office", { name: "Sede" }, OFFICE.modified);
  });

  it("leaving_with_save_stays_when_the_save_is_refused_and_the_focus_goes_to_the_field", async () => {
    const user = userEvent.setup();
    renderShell();
    await typeInto(user, "Name", "");
    await user.click(screen.getByRole("option", { name: /Homelab/ }));
    await user.click(await screen.findByRole("button", { name: "Save" }));
    const name = await screen.findByRole("textbox", { name: "Name" });
    await waitFor(() => expect(document.activeElement).toBe(name));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Ufficio Napoli");
  });

  it("choosing_the_profile_that_is_already_chosen_does_not_ask", async () => {
    const user = userEvent.setup();
    renderShell();
    await typeInto(user, "Name", "Sede");
    await user.click(screen.getByRole("option", { name: /Ufficio Napoli/ }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("a_group_made_in_the_manage_groups_window_is_in_the_group_selector_and_the_list", async () => {
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole("button", { name: "Manage groups" }));
    const dialog = await screen.findByRole("dialog", { name: "Manage groups" });
    await user.type(within(dialog).getByRole("textbox", { name: "New group" }), "Famiglia{Enter}");
    await waitFor(() => expect(within(dialog).getByRole("textbox", { name: "Group 4" })).toBeTruthy());
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Manage groups" }));
    expect(within(screen.getByRole("combobox", { name: "Group" })).getAllByRole("option").map((o) => o.textContent)).toEqual(["No group", "Lavoro", "Personale", "Clienti", "Famiglia"]);
  });

  it("renaming_a_group_in_the_window_renames_it_in_the_list_and_in_the_header", async () => {
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole("button", { name: "Manage groups" }));
    const dialog = await screen.findByRole("dialog", { name: "Manage groups" });
    const field = within(dialog).getByRole("textbox", { name: "Group 1" });
    await user.clear(field);
    await user.type(field, "Ufficio{Enter}");
    await waitFor(() => expect(within(dialog).getByRole("textbox", { name: "Group 1" }) as HTMLInputElement).toHaveProperty("value", "Ufficio"));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getAllByRole("group", { name: "Ufficio" }).length).toBeGreaterThan(0);
    expect(screen.queryByRole("group", { name: "Lavoro" })).toBeNull();
    // the chosen profile was in «Lavoro»: its header and its selector follow
    await waitFor(() => expect(document.querySelector(".detail-meta")?.textContent).toContain("Ufficio"));
    expect((screen.getByRole("combobox", { name: "Group" }) as HTMLSelectElement).value).toBe("Ufficio");
  });

  it("the_form_has_no_accessibility_violations_with_the_save_bar_and_an_error", async () => {
    const user = userEvent.setup();
    const { container } = setup(HOMELAB, withGoodKey(makeDetail(HOMELAB)));
    await typeInto(user, "Name", "");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await screen.findByText("This field is required.");
    await expectNoAccessibilityViolations(container);
  });
});
