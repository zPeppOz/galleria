import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { ToastProvider, useToast } from "./Toast";

function Trigger({ tone }: { tone?: "info" | "ok" | "error" }) {
  const toast = useToast();
  return (
    <button type="button" onClick={() => toast("Saved the profile", tone ? { tone } : undefined)}>
      Notify
    </button>
  );
}

const show = (tone?: "info" | "ok" | "error") =>
  render(
    <ToastProvider closeLabel="Dismiss">
      <main>
        <Trigger tone={tone} />
      </main>
    </ToastProvider>,
  );

describe("Toast", () => {
  it("message_is_announced_in_a_live_region", async () => {
    const user = userEvent.setup();
    show();
    // The region is in the document before the message: a screen reader only announces what is added to a region it knows.
    const region = screen.getByRole("status");
    expect(region.textContent).toBe("");
    await user.click(screen.getByRole("button", { name: "Notify" }));
    expect(region.textContent).toContain("Saved the profile");
    expect(screen.getByRole("status")).toBe(region);
  });

  it("error_tone_uses_an_assertive_region", async () => {
    const user = userEvent.setup();
    show("error");
    const alert = screen.getByRole("alert");
    const status = screen.getByRole("status");
    await user.click(screen.getByRole("button", { name: "Notify" }));
    expect(alert.textContent).toContain("Saved the profile");
    expect(status.textContent).toBe("");
  });

  describe("time", () => {
    beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: false }));
    afterEach(() => vi.useRealTimers());

    const press = () => act(() => { screen.getByRole("button", { name: "Notify" }).click(); });
    const wait = (ms: number) => act(() => { vi.advanceTimersByTime(ms); });

    it("toast_leaves_by_itself_and_pauses_while_focused_or_hovered", () => {
      show();
      press();
      expect(screen.getByRole("status").textContent).toContain("Saved the profile");
      wait(2000);
      expect(screen.getByRole("status").textContent).toContain("Saved the profile");
      wait(4000);
      expect(screen.getByRole("status").textContent).toBe("");

      // hovering holds it past the time it would have gone
      press();
      const toast = screen.getByText("Saved the profile").closest(".gx-toast") as HTMLElement;
      act(() => { toast.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); });
      wait(20000);
      expect(screen.getByRole("status").textContent).toContain("Saved the profile");
      act(() => { toast.dispatchEvent(new MouseEvent("mouseout", { bubbles: true })); });
      wait(6000);
      expect(screen.getByRole("status").textContent).toBe("");

      // so does the focus inside it
      press();
      const close = screen.getByRole("button", { name: "Dismiss" });
      act(() => { close.focus(); });
      wait(20000);
      expect(screen.getByRole("status").textContent).toContain("Saved the profile");
      act(() => { close.blur(); });
      wait(6000);
      expect(screen.getByRole("status").textContent).toBe("");
    });
  });

  it("toast_can_be_dismissed_from_the_keyboard", async () => {
    const user = userEvent.setup();
    show();
    const opener = screen.getByRole("button", { name: "Notify" });
    opener.focus();
    await user.keyboard("{Enter}");
    // the focus stays where it was: the toast never takes it
    expect(document.activeElement).toBe(opener);
    await user.tab();
    const close = screen.getByRole("button", { name: "Dismiss" });
    expect(document.activeElement).toBe(close);
    await user.keyboard("{Enter}");
    expect(screen.getByRole("status").textContent).toBe("");
    // closing it from the keyboard gives the focus back to what had it, not to the page body
    expect(document.activeElement).toBe(opener);
  });

  it("has_no_accessibility_violations", async () => {
    const user = userEvent.setup();
    const { container } = show("error");
    await user.click(screen.getByRole("button", { name: "Notify" }));
    await expectNoAccessibilityViolations(container);
  });
});
