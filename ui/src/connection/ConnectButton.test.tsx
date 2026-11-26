import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { VPN_STATES, type VpnState } from "../daemon/types";
import { backend, renderConnected } from "../test/connection";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeStatus, makeView } from "../test/fixtures";
import { ConnectButton } from "./ConnectButton";

const PROFILE = { id: "office", name: "Ufficio Napoli" };
const session = (state: VpnState, id = PROFILE.id) => makeView({ status: makeStatus({ state, session: 4, profile: { id, name: "X" } }) });
const names = () => screen.getAllByRole("button").map((b) => b.textContent);

describe("ConnectButton", () => {
  it.each<[VpnState, string[]]>([
    ["disconnected", ["Connect"]],
    ["connecting", ["Cancel"]],
    ["needs_input", ["Cancel"]],
    ["connected", ["Disconnect"]],
    ["reconnecting", ["Cancel"]],
    ["disconnecting", ["Disconnecting"]],
    ["error", ["Retry", "Close"]],
  ])("the_button_names_the_action_for_each_of_the_seven_states: %s", (state, expected) => {
    backend();
    renderConnected(<ConnectButton profile={PROFILE} />, session(state));
    expect(names()).toEqual(expected);
    // the seven are all covered
    expect(VPN_STATES).toContain(state);
  });

  it("the_button_of_another_profile_still_connects", () => {
    backend();
    renderConnected(<ConnectButton profile={{ id: "other", name: "Altro" }} />, session("connected"));
    expect(names()).toEqual(["Connect"]);
    expect((screen.getByRole("button") as HTMLButtonElement).disabled).toBe(false);
  });

  it("the_button_follows_the_events_not_the_answer", async () => {
    const user = userEvent.setup();
    const calls = backend();
    const view = renderConnected(<ConnectButton profile={PROFILE} />, makeView());
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["connect_profile"]));
    expect(calls[0].args).toEqual({ id: "office", replace: false });
    // the call was answered, and no event has come: the button is busy, it does not say «Disconnect»
    const busy = screen.getByRole("button");
    expect(busy.getAttribute("aria-busy")).toBe("true");
    expect(busy.textContent).toBe("Connecting");
    expect(screen.queryByRole("button", { name: "Disconnect" })).toBeNull();
    // the events say what it is
    view.show(session("connecting"));
    expect(names()).toEqual(["Cancel"]);
    view.show(session("connected"));
    expect(names()).toEqual(["Disconnect"]);
  });

  it("a_call_that_fails_gives_the_button_back", async () => {
    const user = userEvent.setup();
    backend(() => {
      throw { code: "gui_secret_missing", message: "x" };
    });
    renderConnected(<ConnectButton profile={PROFILE} />, makeView());
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(screen.getByRole("button").getAttribute("aria-busy")).toBeNull());
    expect(names()).toEqual(["Connect"]);
  });

  it("cancel_while_connecting_is_a_disconnect", async () => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<ConnectButton profile={PROFILE} />, session("connecting"));
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["disconnect"]));
  });

  it("dismiss_on_error_is_a_disconnect", async () => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<ConnectButton profile={PROFILE} />, session("error"));
    await user.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["disconnect"]));
  });

  it("retry_after_an_error_connects_the_same_profile_again", async () => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<ConnectButton profile={PROFILE} />, session("error"));
    await user.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["connect_profile"]));
    expect(calls[0].args).toEqual({ id: "office", replace: false });
  });

  it.each([
    ["daemon_not_running", "The service is not running"],
    ["permission_denied", "No access to the service"],
    ["connecting", "Connecting to the service…"],
  ] as const)("without_the_daemon_the_button_is_disabled_and_says_why: %s", async (link, reason) => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<ConnectButton profile={PROFILE} />, makeView({ link, status: null }));
    const button = screen.getByRole("button", { name: "Connect" }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    const why = document.getElementById(button.getAttribute("aria-describedby") ?? "none");
    expect(why?.textContent).toBe(reason);
    await user.click(button);
    expect(calls).toEqual([]);
  });

  it("the_text_is_in_both_languages", () => {
    backend();
    renderConnected(<ConnectButton profile={PROFILE} />, session("connected"), "it");
    expect(names()).toEqual(["Disconnetti"]);
  });

  it("has_no_accessibility_violations", async () => {
    backend();
    for (const state of VPN_STATES) {
      const { container, unmount } = renderConnected(<ConnectButton profile={PROFILE} />, session(state));
      await expectNoAccessibilityViolations(container);
      unmount();
    }
    const { container } = renderConnected(<ConnectButton profile={PROFILE} />, makeView({ link: "daemon_not_running", status: null }));
    await act(async () => undefined);
    await expectNoAccessibilityViolations(container);
  });
});
