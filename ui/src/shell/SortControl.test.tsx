import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import type { SortOrder } from "../app/uiState";
import { ToastProvider } from "../design/Toast";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { SortControl } from "./SortControl";

function Harness({ start = "name" }: { start?: SortOrder }) {
  const [sort, setSort] = useState<SortOrder>(start);
  return (
    <I18n lang="en">
      <ToastProvider closeLabel="Dismiss the notice">
        <SortControl value={sort} onChange={setSort} />
      </ToastProvider>
    </I18n>
  );
}

describe("SortControl", () => {
  it("starts_from_the_saved_order", () => {
    mockIPC(() => undefined);
    render(<Harness start="recent" />);
    expect((screen.getByRole("radio", { name: "Most recent" }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole("radio", { name: "By name" }) as HTMLInputElement).checked).toBe(false);
    expect(screen.getByRole("radiogroup", { name: "Sort order" })).toBeTruthy();
  });

  it("choice_is_saved_through_ui_state_set", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return undefined;
    });
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("radio", { name: "Most recent" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "ui_state_set")).toBe(true));
    expect(calls.find((c) => c.cmd === "ui_state_set")?.args).toEqual({ patch: { sort: "recent" } });
    expect((screen.getByRole("radio", { name: "Most recent" }) as HTMLInputElement).checked).toBe(true);
  });

  it("a_failed_write_keeps_the_choice_and_says_so", async () => {
    mockIPC((cmd) => (cmd === "ui_state_set" ? Promise.reject({ code: "gui_state_not_saved", message: "x" }) : undefined));
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole("radio", { name: "Most recent" }));
    await waitFor(() => expect(document.body.textContent).toContain("Could not save the setting"));
    expect((screen.getByRole("radio", { name: "Most recent" }) as HTMLInputElement).checked).toBe(true);
  });

  it("has_no_accessibility_violations", async () => {
    mockIPC(() => undefined);
    const { container } = render(<main><Harness /></main>);
    await expectNoAccessibilityViolations(container);
  });
});
