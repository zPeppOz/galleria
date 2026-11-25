import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useMemo, useRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../design/Toast";
import { I18n } from "../i18n";
import type { CmdError, GroupActions, ProfileSummary } from "../rubrica/types";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeSummary, SAMPLE_PROFILES } from "../test/fixtures";
import { GroupsDialog } from "./GroupsDialog";

type Fail = Partial<Record<"create" | "rename" | "remove" | "reorder", CmdError>>;

/** The dialog with actions that keep their own list, like the backend does: each answers with the new list. */
function Harness({ start = ["Lavoro", "Personale", "Clienti"], profiles = SAMPLE_PROFILES, fail = {}, announce }: { start?: string[]; profiles?: ProfileSummary[]; fail?: Fail; announce?: (text: string, assertive: boolean) => void }) {
  const [groups, setGroups] = useState(start);
  const [open, setOpen] = useState(false);
  const now = useRef(groups);
  now.current = groups;
  // the same functions on every render, so a test can read the calls they got
  const actions = useMemo<GroupActions>(() => {
    const run = (kind: keyof Fail, change: (list: string[]) => string[]) => async () => {
      const failure = fail[kind];
      if (failure) throw failure;
      const next = change(now.current);
      setGroups(next);
      return next;
    };
    return {
      create: vi.fn(async (name: string) => run("create", (list) => [...list, name.trim()])()),
      rename: vi.fn(async (from: string, to: string) => run("rename", (list) => list.map((g) => (g === from ? to.trim() : g)))()),
      remove: vi.fn(async (name: string) => run("remove", (list) => list.filter((g) => g !== name))()),
      reorder: vi.fn(async (names: string[]) => run("reorder", () => names)()),
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  (Harness as unknown as { last: GroupActions }).last = actions;
  return (
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice" announce={announce}>
        <main>
          <button type="button" onClick={() => setOpen(true)}>Manage groups</button>
          <GroupsDialog open={open} onClose={() => setOpen(false)} groups={groups} profiles={profiles} actions={actions} />
        </main>
      </ToastProvider>
    </I18n>
  );
}
const actionsOf = () => (Harness as unknown as { last: GroupActions }).last;

async function open(props: Parameters<typeof Harness>[0] = {}) {
  const user = userEvent.setup();
  const view = render(<Harness {...props} />);
  await user.click(screen.getByRole("button", { name: "Manage groups" }));
  const dialog = await screen.findByRole("dialog", { name: "Manage groups" });
  return { user, dialog, ...view };
}

const rowNames = (dialog: HTMLElement) => within(dialog).getAllByRole("textbox", { name: /^Group \d+$/ }).map((el) => (el as HTMLInputElement).value);
const toasts = () => document.querySelector<HTMLElement>(".gx-toasts [role=status]")!;
const alerts = () => document.querySelector<HTMLElement>(".gx-toasts [role=alert]")!;

describe("GroupsDialog", () => {
  it("lists_groups_with_their_controls", async () => {
    const { dialog } = await open();
    expect(rowNames(dialog)).toEqual(["Lavoro", "Personale", "Clienti"]);
    for (const name of ["Lavoro", "Personale", "Clienti"]) {
      expect(within(dialog).getByRole("button", { name: `Move ${name} up` })).toBeTruthy();
      expect(within(dialog).getByRole("button", { name: `Move ${name} down` })).toBeTruthy();
      expect(within(dialog).getByRole("button", { name: `Delete ${name}` })).toBeTruthy();
    }
    // the first cannot go up and the last cannot go down
    expect((within(dialog).getByRole("button", { name: "Move Lavoro up" }) as HTMLButtonElement).disabled).toBe(true);
    expect((within(dialog).getByRole("button", { name: "Move Clienti down" }) as HTMLButtonElement).disabled).toBe(true);
    expect((within(dialog).getByRole("button", { name: "Move Personale up" }) as HTMLButtonElement).disabled).toBe(false);
    // how many profiles each has is in words, and the new group is the last thing
    expect(within(dialog).getAllByText("2 profiles")).toHaveLength(2); // Lavoro and Personale
    expect(within(dialog).getByText("1 profile")).toBeTruthy(); // Clienti
    expect(within(dialog).getByRole("textbox", { name: "New group" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Add group" })).toBeTruthy();
  });

  it("an_empty_list_says_so", async () => {
    const { dialog } = await open({ start: [], profiles: [] });
    expect(within(dialog).getByText("No groups yet.")).toBeTruthy();
    expect(within(dialog).getByRole("textbox", { name: "New group" })).toBeTruthy();
  });

  it("new_group_appears_at_the_end_and_gets_the_focus", async () => {
    const { user, dialog } = await open();
    await user.type(within(dialog).getByRole("textbox", { name: "New group" }), "  Famiglia  ");
    await user.click(within(dialog).getByRole("button", { name: "Add group" }));
    await waitFor(() => expect(rowNames(dialog)).toEqual(["Lavoro", "Personale", "Clienti", "Famiglia"]));
    expect(actionsOf().create).toHaveBeenCalledWith("  Famiglia  ");
    // the focus is on the name of the new row, the field to add one is empty again, and the notice says it
    await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole("textbox", { name: "Group 4" })));
    expect((within(dialog).getByRole("textbox", { name: "New group" }) as HTMLInputElement).value).toBe("");
    await waitFor(() => expect(toasts().textContent).toContain("Group Famiglia created."));
  });

  it("enter_in_the_new_group_field_adds_it", async () => {
    const { user, dialog } = await open();
    await user.type(within(dialog).getByRole("textbox", { name: "New group" }), "Famiglia{Enter}");
    await waitFor(() => expect(rowNames(dialog)).toContain("Famiglia"));
  });

  it("move_keeps_the_focus_on_the_moved_row", async () => {
    const { user, dialog } = await open();
    await user.click(within(dialog).getByRole("button", { name: "Move Personale up" }));
    await waitFor(() => expect(rowNames(dialog)).toEqual(["Personale", "Lavoro", "Clienti"]));
    expect(actionsOf().reorder).toHaveBeenCalledWith(["Personale", "Lavoro", "Clienti"]);
    // Personale is first now: its «up» is off, so the focus is on its «down»; no notice for a move
    await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Move Personale down" })));
    expect(toasts().textContent).toBe("");
    await user.click(within(dialog).getByRole("button", { name: "Move Personale down" }));
    await waitFor(() => expect(rowNames(dialog)).toEqual(["Lavoro", "Personale", "Clienti"]));
    // down again, from the middle: it can still go the same way, so the focus is on the same button
    await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Move Personale down" })));
  });

  it("rename_with_enter_keeps_the_place_and_the_focus_and_says_it", async () => {
    const { user, dialog } = await open();
    const field = within(dialog).getByRole("textbox", { name: "Group 2" });
    await user.clear(field);
    await user.type(field, "Casa{Enter}");
    await waitFor(() => expect(rowNames(dialog)).toEqual(["Lavoro", "Casa", "Clienti"]));
    expect(actionsOf().rename).toHaveBeenCalledWith("Personale", "Casa");
    await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole("textbox", { name: "Group 2" })));
    await waitFor(() => expect(toasts().textContent).toContain("Group renamed to Casa."));
  });

  it("the_rename_button_shows_only_when_the_name_changed", async () => {
    const { user, dialog } = await open();
    expect(within(dialog).queryByRole("button", { name: /^Rename group/ })).toBeNull();
    await user.type(within(dialog).getByRole("textbox", { name: "Group 1" }), "x");
    await user.click(within(dialog).getByRole("button", { name: "Rename group Lavoro" }));
    await waitFor(() => expect(rowNames(dialog)[0]).toBe("Lavorox"));
  });

  it("delete_with_members_asks_and_says_how_many", async () => {
    const { user, dialog } = await open();
    await user.click(within(dialog).getByRole("button", { name: "Delete Lavoro" }));
    const ask = await screen.findByRole("alertdialog", { name: "Delete the group Lavoro?" });
    expect(ask.textContent).toContain("2 profiles stay without a group. The profiles themselves are kept.");
    // the safe choice has the focus; Cancel changes nothing
    const cancel = within(ask).getByRole("button", { name: "Cancel" });
    await waitFor(() => expect(document.activeElement).toBe(cancel));
    await user.click(cancel);
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(rowNames(dialog)).toEqual(["Lavoro", "Personale", "Clienti"]);
    expect(actionsOf().remove).not.toHaveBeenCalled();
    // Delete does it, tells it, and the focus goes to the row that took the place
    await user.click(within(dialog).getByRole("button", { name: "Delete Lavoro" }));
    await user.click(await within(await screen.findByRole("alertdialog")).findByRole("button", { name: "Delete" }));
    await waitFor(() => expect(rowNames(dialog)).toEqual(["Personale", "Clienti"]));
    expect(actionsOf().remove).toHaveBeenCalledWith("Lavoro");
    await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole("textbox", { name: "Group 1" })));
    await waitFor(() => expect(toasts().textContent).toContain("Group Lavoro deleted."));
  });

  it("a_group_of_one_profile_says_one_and_an_empty_one_goes_without_asking", async () => {
    const { user, dialog } = await open({ start: ["Lavoro", "Clienti", "Vuoto"] });
    await user.click(within(dialog).getByRole("button", { name: "Delete Clienti" }));
    expect((await screen.findByRole("alertdialog")).textContent).toContain("1 profile stays without a group. The profile itself is kept.");
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    await user.click(within(dialog).getByRole("button", { name: "Delete Vuoto" }));
    await waitFor(() => expect(rowNames(dialog)).toEqual(["Lavoro", "Clienti"]));
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("a_duplicate_name_puts_the_focus_on_the_field_with_the_error", async () => {
    const { user, dialog } = await open();
    const field = within(dialog).getByRole("textbox", { name: "New group" });
    await user.type(field, "lavoro{Enter}");
    await waitFor(() => expect(document.activeElement).toBe(field));
    expect(field.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById(field.getAttribute("aria-describedby") ?? "")?.textContent).toBe("A group with this name already exists.");
    // the rubrica was not asked: the window already knew, and no notice is made for an error that asks for an answer
    expect(actionsOf().create).not.toHaveBeenCalled();
    expect(toasts().textContent).toBe("");
    expect(alerts().textContent).toBe("");
    // typing takes the error away
    await user.type(field, "x");
    expect(field.getAttribute("aria-invalid")).toBeNull();
  });

  it("a_rename_to_a_name_that_is_taken_or_empty_says_so_on_the_field", async () => {
    const { user, dialog } = await open();
    const first = within(dialog).getByRole("textbox", { name: "Group 1" });
    await user.clear(first);
    await user.type(first, "CLIENTI{Enter}");
    await waitFor(() => expect(first.getAttribute("aria-invalid")).toBe("true"));
    expect(document.getElementById(first.getAttribute("aria-describedby") ?? "")?.textContent).toBe("A group with this name already exists.");
    await user.clear(first);
    await user.keyboard("{Enter}");
    await waitFor(() => expect(document.getElementById(first.getAttribute("aria-describedby") ?? "")?.textContent).toBe("This field is required."));
    expect(actionsOf().rename).not.toHaveBeenCalled();
    // changing only the case of its own name is allowed
    await user.clear(first);
    await user.type(first, "LAVORO{Enter}");
    await waitFor(() => expect(actionsOf().rename).toHaveBeenCalledWith("Lavoro", "LAVORO"));
  });

  it("a_refusal_of_the_backend_for_a_name_shows_on_the_field", async () => {
    const refusal: CmdError = { code: "gui_group_exists", message: "exists" };
    const { user, dialog } = await open({ fail: { create: refusal } });
    const field = within(dialog).getByRole("textbox", { name: "New group" });
    await user.type(field, "Nuovo{Enter}");
    await waitFor(() => expect(document.activeElement).toBe(field));
    expect(document.getElementById(field.getAttribute("aria-describedby") ?? "")?.textContent).toBe("A group with this name already exists.");
    expect(alerts().textContent).toBe("");
  });

  it("a_failure_that_is_not_about_a_name_is_an_error_notice", async () => {
    const announce = vi.fn();
    const { user, dialog } = await open({ fail: { reorder: { code: "gui_store_unreadable", message: "x" } }, announce });
    await user.click(within(dialog).getByRole("button", { name: "Move Personale up" }));
    await waitFor(() => expect(alerts().textContent).toContain("The profiles cannot be read"));
    expect(announce).toHaveBeenCalledWith("The profiles cannot be read", true);
    expect(rowNames(dialog)).toEqual(["Lavoro", "Personale", "Clienti"]);
  });

  it("a_group_only_the_profiles_have_is_offered_for_the_list", async () => {
    const profiles = [...SAMPLE_PROFILES, makeSummary({ id: "guest", name: "Ospite", group: "Ospiti" })];
    const { user, dialog } = await open({ profiles });
    expect(within(dialog).getByText("In use by profiles but not in your list")).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Add Ospiti to the list" }));
    await waitFor(() => expect(rowNames(dialog)).toEqual(["Lavoro", "Personale", "Clienti", "Ospiti"]));
    expect(actionsOf().create).toHaveBeenCalledWith("Ospiti");
    expect(within(dialog).queryByRole("button", { name: "Add Ospiti to the list" })).toBeNull();
    await waitFor(() => expect(toasts().textContent).toContain("Ospiti added to the list."));
  });

  it("escape_closes_and_returns_focus_to_the_opener", async () => {
    const { user } = await open();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Manage groups" }));
  });

  it("has_no_accessibility_violations", async () => {
    const profiles = [...SAMPLE_PROFILES, makeSummary({ id: "guest", name: "Ospite", group: "Ospiti" })];
    const { user, dialog } = await open({ profiles });
    await user.type(within(dialog).getByRole("textbox", { name: "New group" }), "lavoro{Enter}");
    await screen.findByText("A group with this name already exists.");
    await expectNoAccessibilityViolations(dialog.parentElement ?? dialog, { landmarks: false });
  });
});
