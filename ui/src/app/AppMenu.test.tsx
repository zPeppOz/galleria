import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App";
import { makeInfo, makeView } from "../test/fixtures";

function backend() {
  mockIPC((cmd) => (cmd === "daemon_view" ? makeView() : cmd === "app_info" ? makeInfo({ version: "9.8.7" }) : undefined), { shouldMockEvents: true });
}

describe("AppMenu", () => {
  it("about_item_opens_the_about_dialog", async () => {
    backend();
    const user = userEvent.setup();
    render(<App lang="en" />);
    await screen.findByText(/App 9\.8\.7/);
    await user.tab(); // «Import…» comes first
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Menu" }));
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    await waitFor(() => expect(document.activeElement?.getAttribute("role")).toBe("menuitemradio"));
    await user.keyboard("{End}{Enter}");
    const dialog = await screen.findByRole("dialog", { name: "About Galleria" });
    expect(within(dialog).getByText("9.8.7")).toBeTruthy();
    expect(within(dialog).getByText("/run/galleria/galleriad.sock")).toBeTruthy();
    expect(within(dialog).getByText("0.1.0")).toBeTruthy(); // the service version, from daemon_view
    // Closing it with Escape puts the focus back on the button «Menu» (D4).
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Menu" }));
  });

  it("language_choice_changes_the_texts", async () => {
    backend();
    const user = userEvent.setup();
    render(<App lang="en" />);
    await screen.findByText(/App 9\.8\.7/);
    await user.tab(); // «Import…» comes first
    await user.tab();
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    expect(screen.getByRole("menuitem", { name: "About Galleria" })).toBeTruthy();
    await user.click(screen.getByRole("menuitemradio", { name: "Italiano" }));
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(screen.getByRole("button", { name: "Menu" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Menu" }));
    expect(await screen.findByRole("menuitem", { name: "Informazioni su Galleria" })).toBeTruthy();
    expect(document.documentElement.lang).toBe("it");
  });
});

describe("the switch of the version check (plan B6, T5)", () => {
  /** The service answers the view that Rust keeps; the patches the page writes change it. */
  function rustWithTheCheck() {
    let enabled = true;
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC(
      (cmd, args) => {
        calls.push({ cmd, args });
        if (cmd === "daemon_view") return makeView();
        if (cmd === "app_info") return makeInfo({ version: "9.8.7" });
        if (cmd === "update_view") return { current: "9.8.7", latest: enabled ? "9.9.9" : null, url: enabled ? "https://github.com/zPeppOz/galleria/releases/tag/v9.9.9" : null, enabled };
        if (cmd === "ui_state_set") {
          enabled = (args as { patch: { update_check?: boolean } }).patch.update_check ?? enabled;
          return {};
        }
        return undefined;
      },
      { shouldMockEvents: true },
    );
    return calls;
  }

  it("the_check_can_be_turned_off_and_the_choice_is_saved", async () => {
    const calls = rustWithTheCheck();
    const user = userEvent.setup();
    render(<App lang="en" />);
    // the notice is in the window while the check is on
    await screen.findByRole("region", { name: /is available$/ });
    await user.click(await screen.findByRole("button", { name: "Menu" }));
    const item = await screen.findByRole("menuitemcheckbox", { name: "Check for a newer version" });
    // it is ticked, above «Quit» and «About Galleria»
    expect(item.getAttribute("aria-checked")).toBe("true");
    const names = [...document.querySelectorAll('[role^="menuitem"]')].map((entry) => entry.textContent);
    expect(names.slice(-3)).toEqual(["Check for a newer version", "Quit", "About Galleria"]);
    await user.click(item);
    await waitFor(() => expect(calls.filter((c) => c.cmd === "ui_state_set").map((c) => c.args)).toEqual([{ patch: { update_check: false } }]));
    // turning it off takes the notice away, and the menu says it is off
    await waitFor(() => expect(screen.queryByRole("region", { name: /is available$/ })).toBeNull());
    await user.click(screen.getByRole("button", { name: "Menu" }));
    expect((await screen.findByRole("menuitemcheckbox", { name: "Check for a newer version" })).getAttribute("aria-checked")).toBe("false");
  });

  it("about_says_the_latest_version_known_or_that_the_check_is_off", async () => {
    rustWithTheCheck();
    const user = userEvent.setup();
    render(<App lang="en" />);
    await screen.findByText(/App 9\.8\.7/);
    await user.click(screen.getByRole("button", { name: "Menu" }));
    await user.click(await screen.findByRole("menuitem", { name: "About Galleria" }));
    let dialog = await screen.findByRole("dialog", { name: "About Galleria" });
    expect(within(dialog).getByText("Latest known version")).toBeTruthy();
    expect(within(dialog).getByText("9.9.9")).toBeTruthy();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // turned off: the row says so, whatever was known
    await user.click(screen.getByRole("button", { name: "Menu" }));
    await user.click(await screen.findByRole("menuitemcheckbox", { name: "Check for a newer version" }));
    await user.click(await screen.findByRole("button", { name: "Menu" }));
    await user.click(await screen.findByRole("menuitem", { name: "About Galleria" }));
    dialog = await screen.findByRole("dialog", { name: "About Galleria" });
    expect(within(dialog).getByText("Check turned off")).toBeTruthy();
  });
});
