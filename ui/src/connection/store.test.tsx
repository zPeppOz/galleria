import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { CommandError } from "../daemon/commands";
import { backend, renderConnected, type Call } from "../test/connection";
import { makeStatus, makeView } from "../test/fixtures";
import { CommandNotice } from "./CommandNotice";
import { ConnectButton } from "./ConnectButton";
import { connectOrReplace } from "./store";

const alreadyActive = (foreign: boolean, name = "Homelab"): CommandError => ({ code: "already_active", message: "A session is already active", data: { profile: { id: "homelab", name }, foreign } });
const toasts = () => document.querySelector<HTMLElement>(".gx-toasts [role=status]")!;

describe("connectOrReplace", () => {
  it("a_session_of_the_rubrica_is_replaced_with_a_second_call_and_the_name_of_the_one_left", async () => {
    const asked: [string, boolean][] = [];
    const outcome = await connectOrReplace(async (id, replace) => {
      asked.push([id, replace]);
      if (!replace) throw alreadyActive(false);
    }, "office");
    expect(asked).toEqual([["office", false], ["office", true]]);
    expect(outcome).toEqual({ replaced: "Homelab" });
  });

  it("a_connection_with_nothing_in_the_way_is_one_call", async () => {
    const asked: boolean[] = [];
    expect(await connectOrReplace(async (_id, replace) => void asked.push(replace), "office")).toEqual({ replaced: null });
    expect(asked).toEqual([false]);
  });

  it("a_foreign_session_is_never_replaced_without_the_confirmation", async () => {
    const asked: boolean[] = [];
    await expect(
      connectOrReplace(async (_id, replace) => {
        asked.push(replace);
        throw alreadyActive(true);
      }, "office"),
    ).rejects.toMatchObject({ code: "already_active", data: { foreign: true } });
    expect(asked).toEqual([false]);
  });

  it("another_error_goes_up_and_nothing_is_retried", async () => {
    const asked: boolean[] = [];
    await expect(
      connectOrReplace(async (_id, replace) => {
        asked.push(replace);
        throw { code: "gui_keyring_locked", message: "locked" };
      }, "office"),
    ).rejects.toMatchObject({ code: "gui_keyring_locked" });
    expect(asked).toEqual([false]);
  });
});

describe("the connection of the window", () => {
  it("an_own_session_is_replaced_at_once_with_one_notice", async () => {
    const user = userEvent.setup();
    const calls = backend((call: Call) => {
      if (call.cmd === "connect_profile" && call.args.replace === false) throw alreadyActive(false, "Homelab");
      return undefined;
    });
    renderConnected(<ConnectButton profile={{ id: "office", name: "Ufficio Napoli" }} />, makeView({ status: makeStatus({ state: "connected", session: 3, profile: { id: "homelab", name: "Homelab" } }) }));
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(calls.map((c) => [c.cmd, c.args.replace])).toEqual([["connect_profile", false], ["connect_profile", true]]));
    // one notice, naming the session that was left
    await waitFor(() => expect(within(toasts()).getAllByText("Disconnected from «Homelab»")).toHaveLength(1));
    expect(within(toasts()).getAllByText(/./)).toHaveLength(1);
  });

  it("a_foreign_session_is_shown_as_an_error_in_line_for_now", async () => {
    const user = userEvent.setup();
    const calls = backend((call: Call) => {
      if (call.cmd === "connect_profile") throw alreadyActive(true, "Altro utente");
      return undefined;
    });
    renderConnected(
      <>
        <ConnectButton profile={{ id: "office", name: "Ufficio Napoli" }} />
        <CommandNotice profileId="office" />
      </>,
      makeView({ status: makeStatus({ state: "connected", session: 3, profile: { id: "other", name: "Altro utente" } }) }),
    );
    await user.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => expect(document.querySelector("[data-notice=command]")).not.toBeNull());
    expect(calls.filter((c) => c.cmd === "connect_profile")).toHaveLength(1);
    expect(document.querySelector("[data-notice=command]")?.textContent).toContain("A VPN is already active");
  });

  it("a_command_error_takes_the_focus_and_keeps_the_state", async () => {
    const user = userEvent.setup();
    backend((call: Call) => {
      if (call.cmd === "connect_profile") throw { code: "gui_secret_missing", message: "x" };
      return undefined;
    });
    const view = makeView();
    renderConnected(
      <>
        <ConnectButton profile={{ id: "office", name: "Ufficio Napoli" }} />
        <CommandNotice profileId="office" />
        <CommandNotice profileId="elsewhere" />
      </>,
      view,
    );
    await user.click(screen.getByRole("button", { name: "Connect" }));
    const notice = (await waitFor(() => document.querySelector("[data-notice=command]")!.querySelector(".gx-alert"))) as HTMLElement;
    expect(notice.textContent).toContain("A key of this profile is missing");
    // the notice takes the focus and it is not a toast: nothing is said twice
    await waitFor(() => expect(document.activeElement).toBe(notice.closest("[data-notice]")));
    expect(document.querySelector(".gx-toasts [role=alert]")?.textContent ?? "").toBe("");
    // another profile has no notice, and the state is as the events left it
    expect(document.querySelectorAll("[data-notice=command]")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Connect" })).toBeTruthy();
  });
});
