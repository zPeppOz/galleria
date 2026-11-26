import { emit } from "@tauri-apps/api/event";
import { act, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CommandNotice } from "../connection/CommandNotice";
import { ReplaceConfirm } from "../connection/ReplaceConfirm";
import type { CommandError } from "../daemon/commands";
import { backend, renderConnected, type Call } from "../test/connection";
import { makeStatus, makeView } from "../test/fixtures";
import { useTrayActions } from "./tray";

const alreadyActive = (foreign: boolean, name = "Homelab"): CommandError => ({ code: "already_active", message: "A session is already active", data: { profile: { id: "homelab", name }, foreign } });
const toasts = () => document.querySelector<HTMLElement>(".gx-toasts [role=status]");

function Listener() {
  useTrayActions();
  return null;
}

const asks = (calls: Call[]) => calls.filter((c) => ["connect_profile", "disconnect", "window_show"].includes(c.cmd)).map((c) => [c.cmd, c.args.replace ?? null]);
const send = (payload: unknown) => act(async () => emit("galleria:tray-action", payload));
const connectedTo = (id: string, name: string) => makeView({ status: makeStatus({ state: "connected", session: 3, profile: { id, name } }) });

describe("the actions of the menu of the tray, done by the page (plan B6, T2, Q52)", () => {
  it("a_tray_connect_runs_the_same_path_as_the_button", async () => {
    const calls = backend();
    renderConnected(<Listener />);
    await send({ action: "connect", id: "office" });
    await waitFor(() => expect(asks(calls)).toEqual([["connect_profile", false]]));
    expect(calls.find((c) => c.cmd === "connect_profile")?.args).toEqual({ id: "office", replace: false });
    // nothing needed the person: the window is not asked to show
    expect(calls.some((c) => c.cmd === "window_show")).toBe(false);
  });

  it("a_tray_connect_over_a_session_of_ours_replaces_it_with_one_notice_and_does_not_show_the_window", async () => {
    const calls = backend((call) => {
      if (call.cmd === "connect_profile" && call.args.replace === false) throw alreadyActive(false, "Homelab");
      return undefined;
    });
    renderConnected(<Listener />, connectedTo("homelab", "Homelab"));
    await send({ action: "connect", id: "office" });
    await waitFor(() => expect(asks(calls)).toEqual([["connect_profile", false], ["connect_profile", true]]));
    await waitFor(() => expect(toasts()?.textContent).toContain("Disconnected from «Homelab»"));
    expect(calls.some((c) => c.cmd === "window_show")).toBe(false);
  });

  it("a_foreign_session_is_never_closed_from_the_menu_without_the_confirmation", async () => {
    const calls = backend();
    // the rubrica has `office` and not `other`: the session of `other` is not ours
    renderConnected(
      <>
        <Listener />
        <ReplaceConfirm />
      </>,
      connectedTo("other", "Altro utente"),
      "en",
      ["office"],
    );
    await send({ action: "disconnect" });
    await waitFor(() => expect(calls.some((c) => c.cmd === "window_show")).toBe(true));
    // the question is in the window, and nothing was ended before it was answered
    expect(calls.some((c) => c.cmd === "disconnect")).toBe(false);
    expect(await screen.findByRole("alertdialog")).toBeTruthy();
  });

  it("a_connect_that_finds_a_foreign_session_asks_in_the_window_and_replaces_nothing", async () => {
    const calls = backend((call) => {
      if (call.cmd === "connect_profile") throw alreadyActive(true, "Altro utente");
      return undefined;
    });
    renderConnected(
      <>
        <Listener />
        <ReplaceConfirm />
      </>,
      connectedTo("other", "Altro utente"),
      "en",
      ["office"],
    );
    await send({ action: "connect", id: "office" });
    await waitFor(() => expect(calls.some((c) => c.cmd === "window_show")).toBe(true));
    expect(calls.filter((c) => c.cmd === "connect_profile")).toHaveLength(1);
    expect(await screen.findByRole("alertdialog")).toBeTruthy();
  });

  it("an_own_session_is_disconnected_from_the_menu_without_a_question", async () => {
    const calls = backend();
    renderConnected(<Listener />, connectedTo("homelab", "Homelab"), "en", ["homelab"]);
    await send({ action: "disconnect" });
    await waitFor(() => expect(asks(calls)).toEqual([["disconnect", null]]));
  });

  it("a_command_error_from_the_menu_shows_the_window", async () => {
    const calls = backend((call) => {
      if (call.cmd === "connect_profile") throw { code: "gui_secret_missing", message: "x" };
      return undefined;
    });
    renderConnected(
      <>
        <Listener />
        <CommandNotice profileId="office" />
      </>,
    );
    await send({ action: "connect", id: "office" });
    await waitFor(() => expect(calls.some((c) => c.cmd === "window_show")).toBe(true));
    // and the error is there for the person to read
    await waitFor(() => expect(document.querySelector("[data-notice=command]")).not.toBeNull());
  });

  it("an_event_that_is_not_an_action_of_the_menu_does_nothing", async () => {
    const calls = backend();
    renderConnected(<Listener />);
    await send({ action: "connect" });
    await send({ action: "format", id: "office" });
    await send("connect");
    await send(null);
    await act(async () => new Promise((resolve) => setTimeout(resolve, 50)));
    expect(asks(calls)).toEqual([]);
  });
});
