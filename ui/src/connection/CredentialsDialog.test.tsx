import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { InputKind, InputRequest } from "../daemon/types";
import { backend, renderConnected } from "../test/connection";
import { expectNoAccessibilityViolations } from "../test/axe";
import { makeStatus, makeView } from "../test/fixtures";
import { CredentialsDialog } from "./CredentialsDialog";

const PASSWORD = "PW-typed-5f3a";

const ask = (over: Partial<InputRequest> = {}, session = 7) =>
  makeView({
    status: makeStatus({
      state: "needs_input",
      session,
      profile: { id: "dc", name: "Datacenter Milano" },
      input_request: { request: 3, kind: "credentials", prompt: "Username and password", echo: false, expires_in_secs: 120, ...over },
    }),
  });

const OTP = { kind: "otp", echo: true, prompt: "Enter OTP" } as const;
const inputs = (box: HTMLElement) => Array.from(box.querySelectorAll<HTMLInputElement>("input")).filter((el) => el.type !== "checkbox");

afterEach(() => vi.useRealTimers());

/** With the clock faked, lets what was just drawn (the portal, the first tick) settle. */
const flush = () =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(50);
  });

describe("CredentialsDialog", () => {
  it.each<[InputKind, string[]]>([
    ["credentials", ["User name", "Password"]],
    ["otp", ["Enter OTP"]],
    ["key_passphrase", ["Passphrase of the key"]],
  ])("only_the_asked_field_is_shown: %s", async (kind, labels) => {
    backend();
    renderConnected(<CredentialsDialog />, ask({ kind, prompt: "Enter OTP", echo: kind === "otp" }));
    const box = await screen.findByRole("dialog");
    expect(inputs(box)).toHaveLength(labels.length);
    for (const label of labels) expect(within(box).getByLabelText(label)).toBeTruthy();
  });

  it("the_name_of_the_profile_is_in_the_title", async () => {
    backend();
    renderConnected(<CredentialsDialog />, ask());
    expect(await screen.findByRole("dialog", { name: "Credentials for «Datacenter Milano»" })).toBeTruthy();
  });

  it("the_server_prompt_is_shown_untranslated", async () => {
    backend();
    renderConnected(<CredentialsDialog />, ask(OTP), "it");
    const box = await screen.findByRole("dialog");
    expect(within(box).getByLabelText("Enter OTP")).toBeTruthy();
    // what the window itself says is in Italian
    expect(within(box).getByRole("button", { name: "Connetti" })).toBeTruthy();
  });

  it("a_field_that_is_not_echoed_is_a_password_field", async () => {
    backend();
    renderConnected(<CredentialsDialog />, ask({ kind: "key_passphrase", echo: false }));
    expect((await screen.findByLabelText("Passphrase of the key")).getAttribute("type")).toBe("password");
  });

  it("remember_is_offered_for_credentials_only", async () => {
    backend();
    const view = renderConnected(<CredentialsDialog />, ask());
    const box = await screen.findByRole("dialog");
    const remember = within(box).getByRole("checkbox", { name: /Remember in the keyring/ }) as HTMLInputElement;
    // off the first time
    expect(remember.checked).toBe(false);
    view.show(ask({ ...OTP, request: 4 }));
    await waitFor(() => expect(screen.queryByRole("checkbox")).toBeNull());
    view.show(ask({ kind: "key_passphrase", request: 5 }));
    await waitFor(() => expect(screen.getByLabelText("Passphrase of the key")).toBeTruthy());
    expect(screen.queryByRole("checkbox")).toBeNull();
  });

  it("the_dialog_takes_the_focus_and_is_not_announced", async () => {
    backend();
    renderConnected(<CredentialsDialog />, ask());
    const box = await screen.findByRole("dialog");
    await waitFor(() => expect(document.activeElement).toBe(within(box).getByLabelText("User name")));
    // nothing in it speaks by itself: no live region, no alert, no status
    expect(box.querySelector('[aria-live], [role="alert"], [role="status"]')).toBeNull();
  });

  it("the_fields_are_empty_before_the_answer_arrives", async () => {
    const user = userEvent.setup();
    const calls = backend((call) => (call.cmd === "provide_input" ? new Promise<void>(() => undefined) : undefined));
    renderConnected(<CredentialsDialog />, ask());
    const username = (await screen.findByLabelText("User name")) as HTMLInputElement;
    const password = screen.getByLabelText("Password") as HTMLInputElement;
    await user.type(username, "carla.v");
    await user.type(password, PASSWORD);
    expect(password.value).toBe(PASSWORD);
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(calls.filter((c) => c.cmd === "provide_input")).toHaveLength(1));
    // the call has not been answered (it never is, here), and what was typed is gone from the page
    expect(username.value).toBe("");
    expect(password.value).toBe("");
    expect(calls[0].args).toEqual({ session: 7, request: 3, values: { username: "carla.v", password: PASSWORD }, remember: false });
  });

  it("remember_travels_with_the_values", async () => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<CredentialsDialog />, ask());
    await user.type(await screen.findByLabelText("User name"), "carla.v");
    await user.type(screen.getByLabelText("Password"), PASSWORD);
    const remember = screen.getByRole("checkbox") as HTMLInputElement;
    await user.click(remember);
    expect(remember.checked).toBe(true);
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].args.remember).toBe(true);
  });

  it("the_fields_are_required", async () => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<CredentialsDialog />, ask(OTP));
    await screen.findByRole("dialog");
    await user.click(screen.getByRole("button", { name: "Connect" }));
    expect(calls).toEqual([]);
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("provide_input_is_invoked_once_per_submit", async () => {
    const calls = backend();
    renderConnected(<CredentialsDialog />, ask(OTP));
    const field = (await screen.findByLabelText("Enter OTP")) as HTMLInputElement;
    field.value = "482913";
    const form = screen.getByTestId("credentials-form");
    // two submits before the page is drawn again: the second one finds the question answered
    act(() => {
      fireEvent.submit(form);
      fireEvent.submit(form);
    });
    await waitFor(() => expect(calls.filter((c) => c.cmd === "provide_input")).toHaveLength(1));
    expect(calls[0].args).toEqual({ session: 7, request: 3, values: { otp: "482913" }, remember: false });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(calls.filter((c) => c.cmd === "provide_input")).toHaveLength(1);
  });

  it("a_new_request_reopens_the_dialog_and_an_answered_one_does_not", async () => {
    const user = userEvent.setup();
    const calls = backend();
    const view = renderConnected(<CredentialsDialog />, ask(OTP));
    await user.type(await screen.findByLabelText("Enter OTP"), "111111");
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // the same question again (an event that repeats the state): not asked twice
    view.show(ask(OTP));
    expect(screen.queryByRole("dialog")).toBeNull();
    // a question that is new, in the same session (a reconnection asks a new code), and then one of another session
    view.show(ask({ ...OTP, request: 4 }));
    expect(await screen.findByRole("dialog")).toBeTruthy();
    await user.type(screen.getByLabelText("Enter OTP"), "222222");
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    view.show(ask({ ...OTP, request: 4 }, 8));
    expect(await screen.findByRole("dialog")).toBeTruthy();
    expect(calls.filter((c) => c.cmd === "provide_input")).toHaveLength(2);
  });

  // The real daemon numbers no request: the OpenVPN backend sends `request: 0` every time (found by G5a in the laboratory, where the
  // window asked for the credentials and then never for the code). A question is new when the state moved on in between, or when its kind
  // is another, whatever its number says.
  it("every_request_numbered_zero_is_asked_in_turn", async () => {
    const user = userEvent.setup();
    const calls = backend();
    const moving = makeView({ status: makeStatus({ state: "connecting", session: 7, profile: { id: "dc", name: "Datacenter Milano" } }) });
    const view = renderConnected(<CredentialsDialog />, ask({ request: 0 }));
    await user.type(await screen.findByLabelText("User name"), "bob");
    await user.type(screen.getByLabelText("Password"), PASSWORD);
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // the code, same session, same number, another kind: the state may not even be seen to move
    view.show(ask({ ...OTP, request: 0 }));
    await user.type(await screen.findByLabelText("Enter OTP"), "111111");
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // a repeated state is not a new question, but the state that moved on and asks for a code again (a reconnection) is
    view.show(ask({ ...OTP, request: 0 }));
    expect(screen.queryByRole("dialog")).toBeNull();
    view.show(moving);
    view.show(ask({ ...OTP, request: 0 }));
    await user.type(await screen.findByLabelText("Enter OTP"), "222222");
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(calls.filter((c) => c.cmd === "provide_input")).toHaveLength(3);
  });

  it("the_dialog_goes_when_the_state_is_no_longer_needs_input", async () => {
    backend();
    const view = renderConnected(<CredentialsDialog />, ask());
    await screen.findByRole("dialog");
    view.show(makeView({ status: makeStatus({ state: "connected", session: 7, profile: { id: "dc", name: "Datacenter Milano" } }) }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("the_countdown_starts_from_the_event", async () => {
    vi.useFakeTimers();
    backend();
    const view = renderConnected(<CredentialsDialog />, ask());
    await flush();
    expect(screen.getByText("Time left: 2:00")).toBeTruthy();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(screen.getByText("Time left: 1:30")).toBeTruthy();
    // drawing the same question again does not start the clock again
    view.show(ask());
    expect(screen.getByText("Time left: 1:30")).toBeTruthy();
    // in the last minute it moves every second
    await act(async () => {
      await vi.advanceTimersByTimeAsync(31_000);
    });
    expect(screen.getByText("Time left: 0:59")).toBeTruthy();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(screen.getByText("Time left: 0:58")).toBeTruthy();
  });

  it("at_zero_the_dialog_closes", async () => {
    vi.useFakeTimers();
    const calls = backend();
    renderConnected(<CredentialsDialog />, ask({ expires_in_secs: 5 }));
    await flush();
    expect(screen.getByRole("dialog")).toBeTruthy();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_500);
    });
    expect(screen.queryByRole("dialog")).toBeNull();
    // the state that follows says the rest: closing is not an answer and not a disconnect
    expect(calls).toEqual([]);
  });

  it("cancel_is_a_disconnect", async () => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<CredentialsDialog />, ask());
    await user.click(await screen.findByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["disconnect"]));
  });

  it("a_click_outside_does_not_close_it", async () => {
    const user = userEvent.setup();
    const calls = backend();
    renderConnected(<CredentialsDialog />, ask());
    await screen.findByRole("dialog");
    const veil = document.querySelector(".gx-backdrop");
    expect(veil).toBeTruthy();
    await user.click(veil as Element);
    await new Promise((resolve) => setTimeout(resolve, 150));
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(calls).toEqual([]);
  });

  it("a_refused_answer_does_not_bring_the_dialog_back", async () => {
    const user = userEvent.setup();
    backend((call) => {
      if (call.cmd === "provide_input") throw { code: "input_not_expected", message: "x" };
      return undefined;
    });
    renderConnected(<CredentialsDialog />, ask(OTP));
    await user.type(await screen.findByLabelText("Enter OTP"), "111111");
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("has_no_accessibility_violations", async () => {
    backend();
    for (const kind of ["credentials", "otp", "key_passphrase"] as const) {
      const { unmount } = renderConnected(<CredentialsDialog />, ask({ kind, echo: kind === "otp", prompt: "Enter OTP" }));
      await screen.findByRole("dialog");
      await expectNoAccessibilityViolations(document.body, { landmarks: false });
      unmount();
    }
  });
});
