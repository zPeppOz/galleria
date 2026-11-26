import { useState } from "react";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderShell } from "../test/shell";
import { makePreview, makeView } from "../test/fixtures";
import type { CmdError } from "../rubrica/types";
import type { ImportActions } from "./types";
import { ImportDialog } from "./ImportDialog";
import { I18n } from "../i18n";
import { ToastProvider } from "../design/Toast";
import { expectNoAccessibilityViolations } from "../test/axe";

/** The window alone, opened by a button, over actions the test gives. */
function Alone({ actions, onImported = () => undefined, announce }: { actions: ImportActions; onImported?: Parameters<typeof ImportDialog>[0]["onImported"]; announce?: (text: string, assertive: boolean) => void }) {
  return (
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice" announce={announce}>
        <Opener actions={actions} onImported={onImported} />
      </ToastProvider>
    </I18n>
  );
}

function Opener({ actions, onImported }: { actions: ImportActions; onImported: Parameters<typeof ImportDialog>[0]["onImported"] }) {
  const [open, setOpen] = useState(false);
  return (
    <main>
      <button type="button" onClick={() => setOpen(true)}>Import…</button>
      <ImportDialog open={open} onClose={() => setOpen(false)} actions={actions} onImported={onImported} />
    </main>
  );
}

function fakeActions(over: Partial<ImportActions> = {}): ImportActions & { fromText: ReturnType<typeof vi.fn>; pickFile: ReturnType<typeof vi.fn>; commit: ReturnType<typeof vi.fn>; discard: ReturnType<typeof vi.fn> } {
  return {
    fromText: vi.fn(async () => makePreview({ suggested_name: "" })),
    pickFile: vi.fn(async () => makePreview({ origin: "picker", file_name: "separate.ovpn", suggested_name: "separate" })),
    commit: vi.fn(async (_id: string, name: string) => ({ id: "new", name, protocol: "openvpn" as const, group: null, favorite: false, server: null, last_used: null, modified: "m" })),
    discard: vi.fn(async () => undefined),
    ...over,
  } as never;
}

const reject = (error: CmdError) => vi.fn(async () => Promise.reject(error));

async function openAlone(actions: ImportActions, extra: { onImported?: Parameters<typeof ImportDialog>[0]["onImported"]; announce?: (text: string, assertive: boolean) => void } = {}) {
  const user = userEvent.setup();
  const view = render(<Alone actions={actions} {...extra} />);
  await user.click(screen.getByRole("button", { name: "Import…" }));
  const dialog = await screen.findByRole("dialog", { name: "Import profile" });
  return { user, dialog, ...view };
}

const textbox = () => screen.getByRole("textbox", { name: "Paste the text of the profile" });
async function paste(user: ReturnType<typeof userEvent.setup>, text = "client") {
  await user.click(textbox());
  await user.paste(text);
}

describe("ImportDialog", () => {
  it("the_button_opens_it_and_the_focus_is_on_the_text", async () => {
    const { dialog } = await openAlone(fakeActions());
    await waitFor(() => expect(document.activeElement).toBe(textbox()));
    expect(within(dialog).getByRole("button", { name: "Preview" })).toHaveProperty("disabled", true);
    // no QR; the file is chosen with the button, in a window that Rust opens
    expect(document.body.textContent).not.toMatch(/QR/);
    expect(within(dialog).getByRole("button", { name: "Choose a file…" })).toHaveProperty("disabled", false);
  });

  it("choose_a_file_shows_the_preview", async () => {
    const actions = fakeActions({ pickFile: vi.fn(async () => makePreview({ origin: "picker", file_name: "separate.ovpn", suggested_name: "separate" })) });
    const { user, dialog } = await openAlone(actions);
    await user.click(within(dialog).getByRole("button", { name: "Choose a file…" }));
    const name = await screen.findByRole("textbox", { name: "Name" });
    await waitFor(() => expect(document.activeElement).toBe(name));
    expect((name as HTMLInputElement).value).toBe("separate");
    expect(dialog.textContent).toContain("File: separate.ovpn");
    expect(actions.pickFile).toHaveBeenCalledTimes(1);
    // the page holds no path: the command took none, and the text box is not part of this step
    expect(actions.fromText).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Import" }));
    await waitFor(() => expect(actions.commit).toHaveBeenCalledWith("import-1", "separate", null));
  });

  it("a_cancelled_choice_keeps_the_focus_on_the_button", async () => {
    const announce = vi.fn();
    const actions = fakeActions({ pickFile: vi.fn(async () => null) });
    const { user, dialog } = await openAlone(actions, { announce });
    const choose = within(dialog).getByRole("button", { name: "Choose a file…" });
    await user.click(choose);
    await waitFor(() => expect(actions.pickFile).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(document.activeElement).toBe(choose));
    // a cancel is not an outcome: still the first step, no alert, no notice, nothing sent to the screen reader
    expect(within(dialog).queryByRole("alert")).toBeNull();
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
    expect(document.querySelectorAll(".gx-toast")).toHaveLength(0);
    expect(announce).not.toHaveBeenCalled();
    expect(actions.discard).not.toHaveBeenCalled();
  });

  it("a_file_that_cannot_be_imported_is_an_alert_with_the_focus", async () => {
    const actions = fakeActions({ pickFile: reject({ code: "gui_import_unsupported_file", message: "x", data: { file_name: "a.txt" } } as CmdError) });
    const { user, dialog } = await openAlone(actions);
    await user.click(within(dialog).getByRole("button", { name: "Choose a file…" }));
    const alert = await within(dialog).findByRole("alert");
    expect(alert.textContent).toContain("Only .ovpn and .conf files can be imported.");
    await waitFor(() => expect(alert.closest(".import-error")?.contains(document.activeElement)).toBe(true));
    expect(document.querySelectorAll(".gx-toast")).toHaveLength(0);
  });

  it("preview_step_focuses_the_name", async () => {
    const { user, dialog } = await openAlone(fakeActions());
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    const name = await screen.findByRole("textbox", { name: "Name" });
    await waitFor(() => expect(document.activeElement).toBe(name));
    // the text of the pasted profile has no name: the page says «Imported profile»
    expect((name as HTMLInputElement).value).toBe("Imported profile");
    expect(within(dialog).getByRole("combobox", { name: "Group" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Import" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Back" })).toBeTruthy();
  });

  it("the_pasted_text_is_cleared_once_the_preview_arrives", async () => {
    const actions = fakeActions();
    const { user, dialog } = await openAlone(actions);
    await paste(user, "client\nremote x 1194\n<key>SEED-SECRET-x</key>");
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    await screen.findByRole("textbox", { name: "Name" });
    expect(actions.fromText).toHaveBeenCalledWith("client\nremote x 1194\n<key>SEED-SECRET-x</key>");
    // nothing of it is in the page any more, not even in a field that is not shown
    expect(document.body.innerHTML).not.toContain("SEED-SECRET");
    await user.click(within(dialog).getByRole("button", { name: "Back" }));
    expect((textbox() as HTMLTextAreaElement).value).toBe("");
  });

  it("an_import_error_takes_the_focus_and_shows_no_notice", async () => {
    const announce = vi.fn();
    const actions = fakeActions({ fromText: reject({ code: "import_invalid", message: "Invalid VPN profile" }) });
    const { user, dialog } = await openAlone(actions, { announce });
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    const alert = await within(dialog).findByRole("alert");
    expect(alert.textContent).toContain("This is not a valid VPN profile.");
    await waitFor(() => expect(alert.closest(".import-error")?.contains(document.activeElement)).toBe(true));
    // an error is read where the focus is: no notice, and nothing sent to the screen reader
    expect(document.querySelectorAll(".gx-toast")).toHaveLength(0);
    expect(announce).not.toHaveBeenCalled();
    // the text is still there to be fixed
    expect((textbox() as HTMLTextAreaElement).value).toBe("client");
  });

  it("a_syntax_error_says_the_line", async () => {
    const actions = fakeActions({ fromText: reject({ code: "import_syntax", message: "Invalid VPN profile at line 4", data: { line: 4 } } as CmdError) });
    const { user, dialog } = await openAlone(actions);
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    expect((await within(dialog).findByRole("alert")).textContent).toContain("there is an error at line 4");
  });

  it("a_file_reference_in_pasted_text_says_to_import_the_file", async () => {
    const actions = fakeActions({ fromText: reject({ code: "import_missing_file", message: "x", data: { file: "ca.crt", origin: "text" } } as CmdError) });
    const { user, dialog } = await openAlone(actions);
    await paste(user, "client\nca ca.crt\n");
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    const text = (await within(dialog).findByRole("alert")).textContent ?? "";
    expect(text).toContain("refers to the file “ca.crt”");
    expect(text).toContain("Import the file itself instead.");
  });

  it("cancel_escape_and_back_discard_the_preview", async () => {
    const actions = fakeActions();
    const { user, dialog } = await openAlone(actions);
    const toPreview = async (box: HTMLElement) => {
      await paste(user, "client");
      await user.click(within(box).getByRole("button", { name: "Preview" }));
      await screen.findByRole("textbox", { name: "Name" });
    };
    await toPreview(dialog);
    await user.click(within(dialog).getByRole("button", { name: "Back" }));
    expect(actions.discard).toHaveBeenCalledTimes(1);
    expect(actions.discard).toHaveBeenLastCalledWith("import-1");
    await toPreview(dialog);
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(actions.discard).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // Escape
    await user.click(screen.getByRole("button", { name: "Import…" }));
    const again = await screen.findByRole("dialog", { name: "Import profile" });
    await toPreview(again);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(actions.discard).toHaveBeenCalledTimes(3);
  });

  it("import_saves_with_the_name_and_the_group_and_says_so_once", async () => {
    const actions = fakeActions();
    const imported = vi.fn();
    const announce = vi.fn();
    const { user, dialog } = await openAlone(actions, { onImported: imported, announce });
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    const name = await screen.findByRole("textbox", { name: "Name" });
    await user.clear(name);
    await user.type(name, "Casa");
    await user.selectOptions(within(dialog).getByRole("combobox", { name: "Group" }), "Personale");
    await user.click(within(dialog).getByRole("button", { name: "Import" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(actions.commit).toHaveBeenCalledWith("import-1", "Casa", "Personale");
    expect(actions.discard).not.toHaveBeenCalled();
    expect(imported).toHaveBeenCalledWith(expect.objectContaining({ name: "Casa" }));
    expect(announce.mock.calls).toEqual([["Imported “Casa”.", false]]);
  });

  it.each([
    [0, "Imported “X”."],
    [1, "Imported “X”. 1 directive removed."],
    [6, "Imported “X”. 6 directives removed."],
  ])("the_notice_says_how_many_directives_were_removed: %i", async (removed, expected) => {
    const announce = vi.fn();
    const actions = fakeActions({ fromText: vi.fn(async () => makePreview({ removed, suggested_name: "X" })) });
    const { user, dialog } = await openAlone(actions, { announce });
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    await screen.findByRole("textbox", { name: "Name" });
    await user.click(within(dialog).getByRole("button", { name: "Import" }));
    await waitFor(() => expect(announce).toHaveBeenCalled());
    // one notice, one announcement
    expect(announce.mock.calls).toEqual([[expected, false]]);
    expect(document.querySelectorAll(".gx-toasts .gx-toast")).toHaveLength(1);
  });

  it("a_refused_name_moves_the_focus_to_the_field", async () => {
    const actions = fakeActions({ commit: reject({ code: "gui_invalid_field", message: "x", data: { fields: [{ field: "name", code: "too_long" }] } } as CmdError) });
    const { user, dialog } = await openAlone(actions);
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    const name = await screen.findByRole("textbox", { name: "Name" });
    await user.click(within(dialog).getByRole("button", { name: "Import" }));
    await waitFor(() => expect(name.getAttribute("aria-invalid")).toBe("true"));
    await waitFor(() => expect(document.activeElement).toBe(name));
    const described = document.getElementById(name.getAttribute("aria-describedby") ?? "");
    expect(described?.textContent).toBeTruthy();
    // the window is still on the preview, which is still held in Rust
    expect(within(dialog).getByRole("button", { name: "Import" })).toBeTruthy();
    expect(actions.discard).not.toHaveBeenCalled();
  });

  it("an_expired_preview_goes_back_and_says_so", async () => {
    const actions = fakeActions({ commit: reject({ code: "gui_import_expired", message: "gone" }) });
    const { user, dialog } = await openAlone(actions);
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    await screen.findByRole("textbox", { name: "Name" });
    await user.click(within(dialog).getByRole("button", { name: "Import" }));
    const alert = await within(dialog).findByRole("alert");
    expect(alert.textContent).toContain("The preview has expired: choose the file again.");
    expect(textbox()).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Name" })).toBeNull();
  });

  it("has_no_accessibility_violations", async () => {
    const { user, dialog } = await openAlone(fakeActions({ fromText: vi.fn(async () => makePreview({ notices: [{ line: 7, directive: "up", kind: "dangerous", dns_script: false }], removed: 1 })) }));
    await expectNoAccessibilityViolations(dialog, { landmarks: false });
    await paste(user);
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    await screen.findByRole("textbox", { name: "Name" });
    await expectNoAccessibilityViolations(dialog, { landmarks: false });
  });
});

describe("the shell and the import", () => {
  it("ctrl_o_and_the_button_open_it_and_focus_the_text", async () => {
    const user = userEvent.setup();
    renderShell(makeView());
    await user.keyboard("{Control>}o{/Control}");
    const dialog = await screen.findByRole("dialog", { name: "Import profile" });
    await waitFor(() => expect(document.activeElement).toBe(within(dialog).getByRole("textbox", { name: "Paste the text of the profile" })));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await user.click(screen.getByRole("button", { name: "Import…" }));
    expect(await screen.findByRole("dialog", { name: "Import profile" })).toBeTruthy();
  });

  it("import_selects_the_new_profile_and_focuses_its_row", async () => {
    const user = userEvent.setup();
    renderShell(makeView());
    await user.click(screen.getByRole("button", { name: "Import…" }));
    const dialog = await screen.findByRole("dialog", { name: "Import profile" });
    await user.click(within(dialog).getByRole("textbox", { name: "Paste the text of the profile" }));
    await user.paste("client");
    await user.click(within(dialog).getByRole("button", { name: "Preview" }));
    const name = await screen.findByRole("textbox", { name: "Name" });
    await user.clear(name);
    await user.type(name, "Appena arrivato");
    await user.click(within(dialog).getByRole("button", { name: "Import" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(await screen.findByRole("heading", { level: 1, name: "Appena arrivato" })).toBeTruthy();
    const row = screen.getByRole("option", { name: /Appena arrivato/ });
    expect(row.getAttribute("aria-selected")).toBe("true");
    await waitFor(() => expect(document.activeElement).toBe(row));
  });
});
