import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { DaemonView } from "./daemon/types";
import { makeDetail, makeInfo, makeRubricaView, makeStatus, makeView } from "./test/fixtures";
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
  const rubrica = makeRubricaView();
  mockIPC(
    (cmd, args) => {
      calls.push(cmd);
      if (cmd === "daemon_view") return view;
      if (cmd === "app_info") return makeInfo();
      if (cmd === "profiles_list") return rubrica;
      if (cmd === "profile_get") return makeDetail(rubrica.profiles.find((p) => p.id === (args as { id: string }).id) ?? rubrica.profiles[0]);
      return undefined;
    },
    { shouldMockEvents: true },
  );
  return calls;
}
// The first answers of the backend: the app version in the bar at the bottom, and the profile with its form.
const ready = async () => {
  await waitFor(() => expect(within(screen.getByRole("contentinfo")).getByText(/App 0\.1\.0/)).toBeTruthy());
  await screen.findByRole("textbox", { name: "Nome" });
};

describe("App", () => {
  it("renders_the_app_title", async () => {
    backend(VIEWS.connected);
    render(<App lang="it" />);
    expect(within(screen.getByRole("banner")).getByText("Galleria")).toBeTruthy();
    await ready();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Ufficio Napoli");
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
    await user.tab(); // «Importa…» of the title bar comes first
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Importa…" }));
    await user.tab(); // then the menu button
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Menu" }));
    await user.tab(); // then the search field, the sort order (one stop) and the list (one stop)
    expect(document.activeElement).toBe(screen.getByRole("searchbox", { name: "Cerca connessioni" }));
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("radio", { name: "Per nome" }));
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("option", { name: "Ufficio Napoli" }));
    await user.tab(); // «Gestisci gruppi» closes the column of the list
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Gestisci gruppi" }));
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
    // every button of the page, the menu and the dialog it opens included
    for (const button of screen.queryAllByRole("button")) {
      // the star writes at once (P7): that is its job, and what it asks for is tried in ProfileDetail.test.tsx
      if (button.getAttribute("aria-haspopup") || button.hasAttribute("aria-pressed")) continue;
      await user.click(button);
    }
    expect(new Set(calls)).toEqual(new Set(["daemon_view", "app_info", "profiles_list", "profile_get", "import_pending", "secret_store_status", "announce_phrases", ...(calls.includes("daemon_retry") ? ["daemon_retry"] : []), ...(calls.includes("connect_profile") ? ["connect_profile"] : [])]));
  });
});
