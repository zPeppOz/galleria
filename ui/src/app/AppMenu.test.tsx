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
