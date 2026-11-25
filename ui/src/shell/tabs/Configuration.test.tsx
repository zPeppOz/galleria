import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../../design/Toast";
import { I18n } from "../../i18n";
import type { CmdError, ProfileActions, ProfileDetail as Detail, ProfilePatch, RawCheck } from "../../rubrica/types";
import { expectNoAccessibilityViolations } from "../../test/axe";
import { applyPatch, makeDetail, SAMPLE_PROFILES } from "../../test/fixtures";
import { ProfileDetail } from "../ProfileDetail";

const [OFFICE, , HOMELAB] = SAMPLE_PROFILES;

type Update = (id: string, patch: ProfilePatch, expectedModified: string) => Promise<Detail>;
type Validate = (id: string, text: string) => Promise<RawCheck>;

/** The tab under test does not use the three actions of «More actions». */
const noActions: ProfileActions = { duplicate: async () => OFFICE, export: async () => false, remove: async () => undefined };

const UP: RawCheck = { can_save: false, issues: [{ line: 2, directive: "up", kind: "dangerous" }, { line: 3, directive: "frob", kind: "unsupported" }] };
const TEXT = "client\nup /tmp/x.sh\nfrob yes\nremote a.example 1194\n";

/** The whole form of a profile, on the tab «Configuration», with the answers of the backend that a test says. */
const setup = (detail: Detail, validate: Validate = async () => ({ issues: [], can_save: true }), update?: Update) => {
  const send = vi.fn<Update>(update ?? (async (_id, patch) => applyPatch(detail, patch)));
  const check = vi.fn<Validate>(validate);
  const announce = vi.fn();
  const view = render(
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice" announce={announce}>
        <main>
          <ProfileDetail profile={detail} detail={detail} detailError={null} groups={["Lavoro"]} update={send} validateRaw={check} actions={noActions} names={[detail.name]} inUse={false} onSelect={() => undefined} onDeleted={() => undefined} reload={async () => undefined} />
        </main>
      </ToastProvider>
    </I18n>,
  );
  return { ...view, send, check, announce };
};

const user = () => userEvent.setup();
const openConfig = async (u: ReturnType<typeof userEvent.setup>) => {
  await u.click(screen.getByRole("tab", { name: "Configuration" }));
  return screen.getByRole("textbox", { name: "Configuration" }) as HTMLTextAreaElement;
};
const type = (area: HTMLTextAreaElement, text: string) => fireEvent.change(area, { target: { value: text } });
const list = () => screen.findByRole("list", { name: "Problems in the configuration" });

describe("Configuration", () => {
  it("the_text_of_an_openvpn_profile_is_an_editor_and_is_not_checked_until_it_changes", async () => {
    const detail = makeDetail(OFFICE);
    const { check } = setup(detail);
    const area = await openConfig(user());
    expect(area.value).toBe(detail.openvpn!.config);
    expect(area.readOnly).toBe(false);
    expect(area.getAttribute("spellcheck")).toBe("false");
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(check).not.toHaveBeenCalled();
  });

  it("the_text_is_checked_400_ms_after_the_last_change", async () => {
    const { check } = setup(makeDetail(OFFICE), async () => UP);
    const area = await openConfig(user());
    type(area, "client\nu");
    type(area, TEXT);
    expect(check).not.toHaveBeenCalled();
    await waitFor(() => expect(check).toHaveBeenCalledTimes(1));
    expect(check).toHaveBeenCalledWith("office", TEXT);
  });

  it("issues_are_listed_with_line_and_directive", async () => {
    setup(makeDetail(OFFICE), async () => UP);
    type(await openConfig(user()), TEXT);
    const issues = within(await list()).getAllByRole("button");
    expect(issues.map((b) => b.textContent)).toEqual([
      "Line 2 · up · Galleria does not run the scripts of profiles.",
      "Line 3 · frob · Galleria does not support this directive.",
    ]);
  });

  it("script_directive_shows_the_d19_text", async () => {
    setup(makeDetail(OFFICE), async () => UP);
    type(await openConfig(user()), TEXT);
    expect(within(await list()).getByRole("button", { name: /Line 2 · up/ }).textContent).toContain("Galleria does not run the scripts of profiles.");
  });

  it("a_dangerous_directive_that_is_not_a_script_does_not_claim_to_be_one", async () => {
    setup(makeDetail(OFFICE), async () => ({ can_save: false, issues: [{ line: 2, directive: "log", kind: "dangerous" }] }));
    type(await openConfig(user()), "client\nlog /tmp/x\n");
    const text = (await within(await list()).findByRole("button")).textContent ?? "";
    expect(text).toContain("Line 2 · log ·");
    expect(text).not.toContain("scripts");
  });

  it("the_text_of_a_syntax_error_has_a_line_and_no_directive", async () => {
    setup(makeDetail(OFFICE), async () => ({ can_save: false, issues: [{ line: 2, directive: "", kind: "syntax" }] }));
    type(await openConfig(user()), 'client\nremote "a\n');
    expect((await within(await list()).findByRole("button")).textContent).toBe("Line 2 · The text cannot be read: check the quotes and the closing tags.");
  });

  it("an_issue_button_moves_the_caret_to_its_line", async () => {
    const u = user();
    setup(makeDetail(OFFICE), async () => UP);
    const area = await openConfig(u);
    type(area, TEXT);
    await u.click(within(await list()).getByRole("button", { name: /Line 3 · frob/ }));
    expect(document.activeElement).toBe(area);
    const start = TEXT.indexOf("frob");
    expect([area.selectionStart, area.selectionEnd]).toEqual([start, start + "frob yes".length]);
    await u.click(within(await list()).getByRole("button", { name: /Line 2 · up/ }));
    expect(area.selectionStart).toBe(TEXT.indexOf("up /tmp"));
  });

  it("the_missing_client_is_a_warning_with_no_line_and_does_not_stop_the_save", async () => {
    const u = user();
    const warning: RawCheck = { can_save: true, issues: [{ line: 0, directive: "tls-client", kind: "no_tls_client" }] };
    const { send } = setup(makeDetail(OFFICE), async () => warning);
    type(await openConfig(u), "remote a.example 1194\n");
    const issues = await list();
    expect(within(issues).queryByRole("button")).toBeNull();
    expect(issues.textContent).toContain("There is no client or tls-client line");
    await u.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][1]).toEqual({ openvpn_config: "remote a.example 1194\n" });
  });

  it("save_is_blocked_while_a_blocking_issue_remains", async () => {
    const u = user();
    const { send, check } = setup(makeDetail(OFFICE), async () => UP);
    type(await openConfig(u), TEXT);
    await u.click(await screen.findByRole("button", { name: "Save" }));
    // the text is checked again right before the save, and nothing is sent
    await waitFor(() => expect(check.mock.calls.length).toBeGreaterThanOrEqual(2));
    expect(send).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();
  });

  it("a_refused_save_moves_the_focus_to_the_first_issue", async () => {
    const u = user();
    const { announce } = setup(makeDetail(OFFICE), async () => UP);
    type(await openConfig(u), TEXT);
    await u.click(await screen.findByRole("button", { name: "Save" }));
    await waitFor(() => expect(document.activeElement).toBe(within(screen.getByRole("list", { name: "Problems in the configuration" })).getAllByRole("button")[0]));
    // the focus is the answer: no notice, so nothing is announced twice
    expect(announce).not.toHaveBeenCalled();
    expect(screen.getByText(/The configuration has problems that stop it from being saved/)).toBeTruthy();
  });

  it("a_refused_save_from_another_tab_opens_the_configuration", async () => {
    const u = user();
    setup(makeDetail(OFFICE), async () => UP);
    type(await openConfig(u), TEXT);
    await u.click(screen.getByRole("tab", { name: "General" }));
    await u.keyboard("{Control>}s{/Control}");
    await waitFor(() => expect(screen.getByRole("tab", { name: "Configuration" }).getAttribute("aria-selected")).toBe("true"));
    await waitFor(() => expect(document.activeElement?.textContent).toContain("Line 2 · up"));
  });

  it("an_edit_takes_the_error_of_the_text_away", async () => {
    const u = user();
    const { check } = setup(makeDetail(OFFICE), async () => UP);
    const area = await openConfig(u);
    type(area, TEXT);
    await u.click(await screen.findByRole("button", { name: "Save" }));
    await screen.findByText(/The configuration has problems that stop it/);
    check.mockImplementation(async () => ({ issues: [], can_save: true }));
    type(area, "client\nremote a.example 1194\n");
    expect(screen.queryByText(/The configuration has problems that stop it/)).toBeNull();
    await waitFor(() => expect(screen.queryByRole("list", { name: "Problems in the configuration" })).toBeNull());
  });

  it("a_refusal_from_rust_shows_its_list_and_the_focus_goes_to_it", async () => {
    const u = user();
    const refusal: CmdError = {
      code: "gui_invalid_field",
      message: "Some fields are not valid",
      data: { fields: [{ field: "openvpn_config", code: "config_issues" }], issues: [{ line: 2, directive: "up", kind: "dangerous" }] },
    };
    const { send, announce } = setup(
      makeDetail(OFFICE),
      async () => ({ issues: [], can_save: true }),
      async () => {
        throw refusal;
      },
    );
    type(await openConfig(u), TEXT);
    await u.click(await screen.findByRole("button", { name: "Save" }));
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    const first = within(await list()).getAllByRole("button")[0];
    await waitFor(() => expect(document.activeElement).toBe(first));
    expect(announce).not.toHaveBeenCalled();
  });

  it("a_clean_text_is_saved_as_typed_with_the_notice", async () => {
    const u = user();
    const { send, announce } = setup(makeDetail(OFFICE));
    const text = "# mine\nclient\n\nremote a.example 1194   # the gateway\n";
    type(await openConfig(u), text);
    await u.click(await screen.findByRole("button", { name: "Save" }));
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][1]).toEqual({ openvpn_config: text });
    await waitFor(() => expect(announce).toHaveBeenCalledWith("Profile saved.", false));
    expect((screen.getByRole("textbox", { name: "Configuration" }) as HTMLTextAreaElement).value).toBe(text);
  });

  it("cancel_brings_the_saved_text_back_and_clears_the_list", async () => {
    const u = user();
    const detail = makeDetail(OFFICE);
    setup(detail, async () => UP);
    const area = await openConfig(u);
    type(area, TEXT);
    await list();
    await u.click(screen.getByRole("button", { name: "Cancel" }));
    expect(area.value).toBe(detail.openvpn!.config);
    expect(screen.queryByRole("list", { name: "Problems in the configuration" })).toBeNull();
  });

  it("wireguard_config_is_read_only_and_has_no_key", async () => {
    const detail = makeDetail(HOMELAB);
    setup(detail);
    const area = await openConfig(user());
    expect(area.readOnly).toBe(true);
    expect(area.value).toBe(detail.wireguard_config);
    expect(area.value).toContain("[Peer]");
    expect(area.value).not.toMatch(/PrivateKey|PresharedKey/);
    expect(screen.getByText(/Read only\. Every setting is in the other tabs/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();
  });

  it("stored_secrets_are_named_below_the_editor", async () => {
    const u = user();
    const office = { ...makeDetail(OFFICE), secret_refs: ["openvpn.block.key", "openvpn.block.tls-crypt", "openvpn.block.something-new"] };
    const { unmount } = setup(office);
    await openConfig(u);
    expect(screen.getByText("In the secrets store: private key, TLS encryption key, openvpn.block.something-new.")).toBeTruthy();
    unmount();

    const home = makeDetail(HOMELAB);
    const two = { ...home, secret_refs: ["wireguard.private_key", "wireguard.peer.1.preshared_key"], wireguard: { ...home.wireguard!, peers: [...home.wireguard!.peers, home.wireguard!.peers[0]] } };
    const second = setup(two);
    await openConfig(u);
    expect(screen.getByText("In the secrets store: private key, shared key of peer 2.")).toBeTruthy();
    second.unmount();

    setup({ ...home, secret_refs: ["wireguard.private_key", "wireguard.peer.0.preshared_key"] });
    await openConfig(u);
    expect(screen.getByText("In the secrets store: private key, shared key.")).toBeTruthy();
  });

  it("no_secret_text_is_among_the_names", async () => {
    setup({ ...makeDetail(OFFICE), secret_refs: ["openvpn.block.key"] });
    await openConfig(user());
    expect(document.body.textContent).not.toMatch(/PRIVATE KEY|BEGIN/);
  });

  it("a_profile_without_secrets_says_so", async () => {
    setup(makeDetail(OFFICE));
    await openConfig(user());
    expect(screen.getByText("No secrets are stored for this profile.")).toBeTruthy();
  });

  it("has_no_accessibility_violations", async () => {
    const u = user();
    const { container, unmount } = setup(makeDetail(OFFICE), async () => UP);
    type(await openConfig(u), TEXT);
    await list();
    await expectNoAccessibilityViolations(container);
    unmount();
    const wireguard = setup(makeDetail(HOMELAB));
    await openConfig(u);
    await expectNoAccessibilityViolations(wireguard.container);
  });
});
