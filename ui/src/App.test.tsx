import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { DaemonView } from "./daemon/types";
import { makeInfo, makeStatus, makeView } from "./test/fixtures";
import { expectNoAccessibilityViolations } from "./test/axe";

const incompatible = { daemon_min: 2, daemon_max: 3, daemon_version: "0.2.0" };
const VIEWS: Record<string, DaemonView> = {
  connecting: makeView({ link: "connecting", status: null }),
  daemon_not_running: makeView({ link: "daemon_not_running", status: null }),
  permission_denied: makeView({ link: "permission_denied", status: null }),
  incompatible_protocol: makeView({ link: "incompatible_protocol", status: null, incompatible }),
  connected: makeView(),
  connected_with_error: makeView({ status: makeStatus({ state: "error", error: { code: "auth_failed", message: "Authentication failed" } }) }),
};

function backend(view: DaemonView) {
  const calls: string[] = [];
  mockIPC((cmd) => { calls.push(cmd); return cmd === "daemon_view" ? view : cmd === "app_info" ? makeInfo() : undefined; }, { shouldMockEvents: true });
  return calls;
}
const ready = () => screen.findByRole("region");

describe("App", () => {
  it("renders_the_app_title", async () => {
    backend(VIEWS.connected);
    render(<App lang="it" />);
    expect(screen.getByRole("heading", { level: 1, name: "Galleria" })).toBeTruthy();
    await ready();
  });

  it.each(Object.keys(VIEWS))("has_no_accessibility_violations: %s", async (name) => {
    backend(VIEWS[name]);
    const { container } = render(<App lang="it" />);
    await ready();
    await expectNoAccessibilityViolations(container);
  });

  it("is_usable_from_the_keyboard", async () => {
    const calls = backend(VIEWS.daemon_not_running);
    const user = userEvent.setup();
    const write = vi.spyOn(navigator.clipboard, "writeText");
    render(<App lang="it" />);
    await ready();
    await user.tab(); // the menu button of the title bar comes first
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Menu" }));
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Copia" }));
    await user.keyboard("{Enter}");
    expect(write).toHaveBeenCalledWith("sudo systemctl enable --now galleriad");
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Riprova" }));
    await user.keyboard("{Enter}");
    await waitFor(() => expect(calls).toContain("daemon_retry"));
  });

  it.each(Object.keys(VIEWS))("nothing_is_executed: %s", async (name) => {
    const calls = backend(VIEWS[name]);
    const user = userEvent.setup();
    render(<App lang="it" />);
    await ready();
    for (const button of screen.queryAllByRole("button")) await user.click(button);
    expect(new Set(calls)).toEqual(new Set(["daemon_view", "app_info", ...(calls.includes("daemon_retry") ? ["daemon_retry"] : [])]));
  });
});
