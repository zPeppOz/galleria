import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App";
import { Preferences } from "../prefs";
import { backend, renderConnected } from "../test/connection";
import { makeDetail, makeInfo, makeRubricaView, makeStatus, makeView } from "../test/fixtures";
import { AppMenu } from "./AppMenu";
import { carryOut, reportUnsaved } from "./closing";

describe("what the page tells Rust about closing (plan B6, T3)", () => {
  it("unsaved_set_is_called_when_dirtiness_changes_not_on_every_key", async () => {
    const calls = backend();
    // ten keys while the form is dirty: the page reports the same bit ten times, Rust is told once
    for (let i = 0; i < 10; i += 1) reportUnsaved(true);
    expect(calls.filter((c) => c.cmd === "unsaved_set").map((c) => c.args)).toEqual([{ dirty: true }]);
    reportUnsaved(false);
    reportUnsaved(false);
    expect(calls.filter((c) => c.cmd === "unsaved_set").map((c) => c.args)).toEqual([{ dirty: true }, { dirty: false }]);
  });

  it("save_then_quit_quits_only_if_the_save_worked_and_discard_carries_out_the_intent", async () => {
    const calls = backend((call) => (call.cmd === "window_hide" ? { done: true } : call.cmd === "app_quit" ? { done: false } : undefined));
    reportUnsaved(true);
    // the bit goes down first, then Rust is asked again by the same decision for what the intent says
    expect(await carryOut("hide")).toBe(true);
    expect(await carryOut("quit")).toBe(false);
    expect(calls.map((c) => c.cmd)).toEqual(["unsaved_set", "unsaved_set", "window_hide", "app_quit"]);
    expect(calls[1].args).toEqual({ dirty: false });
  });

  it("quit_is_in_the_menu_and_on_ctrl_q", async () => {
    const user = userEvent.setup();
    const calls: string[] = [];
    mockIPC(
      (cmd) => {
        calls.push(cmd);
        return cmd === "daemon_view" ? makeView() : cmd === "app_info" ? makeInfo() : cmd === "profiles_list" ? makeRubricaView() : cmd === "profile_get" ? makeDetail(makeRubricaView().profiles[0]) : undefined;
      },
      { shouldMockEvents: true },
    );
    render(<App lang="en" />);
    await screen.findByRole("button", { name: "Menu" });
    await user.keyboard("{Control>}q{/Control}");
    await waitFor(() => expect(calls).toContain("app_quit"));
    calls.length = 0;
    await user.click(screen.getByRole("button", { name: "Menu" }));
    await user.click(await screen.findByRole("menuitem", { name: "Quit" }));
    await waitFor(() => expect(calls).toContain("app_quit"));
  });

  it("quit_says_the_vpn_stays_while_connected", async () => {
    const user = userEvent.setup();
    backend();
    const connected = makeView({ status: makeStatus({ state: "connected", session: 3, profile: { id: "homelab", name: "Homelab" } }) });
    renderConnected(
      <Preferences lang="en">
        <AppMenu info={null} daemonVersion={null} />
      </Preferences>,
      connected,
    );
    await user.click(screen.getByRole("button", { name: "Menu" }));
    expect(await screen.findByRole("menuitem", { name: "Quit (the VPN stays on)" })).toBeTruthy();
  });
});
