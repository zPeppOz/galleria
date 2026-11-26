import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { ConfirmDialog } from "./ConfirmDialog";

const show = (over: Partial<Parameters<typeof ConfirmDialog>[0]> = {}) => {
  const onConfirm = vi.fn();
  const onCancel = vi.fn();
  render(
    <ConfirmDialog open title="Delete Homelab?" body="The profile is removed from the list." confirmLabel="Delete" cancelLabel="Cancel" onConfirm={onConfirm} onCancel={onCancel} {...over} />,
  );
  return { onConfirm, onCancel };
};

describe("ConfirmDialog", () => {
  it("has_alertdialog_role_named_by_title_and_described_by_body", async () => {
    show();
    const dialog = await screen.findByRole("alertdialog", { name: "Delete Homelab?" });
    const id = dialog.getAttribute("aria-describedby");
    expect(id).toBeTruthy();
    expect(document.getElementById(id ?? "")?.textContent).toBe("The profile is removed from the list.");
  });

  it("focus_starts_on_cancel_when_danger", async () => {
    show({ danger: true });
    const cancel = await screen.findByRole("button", { name: "Cancel" });
    await waitFor(() => expect(document.activeElement).toBe(cancel));
  });

  it("focus_starts_on_cancel_when_asked_without_danger", async () => {
    show({ cancelFirst: true });
    const cancel = await screen.findByRole("button", { name: "Cancel" });
    await waitFor(() => expect(document.activeElement).toBe(cancel));
    // the confirm button keeps the look of a plain action
    expect(screen.getByRole("button", { name: "Delete" }).className).toContain("gx-button-primary");
  });

  it("focus_starts_on_confirm_when_not_danger", async () => {
    show();
    const confirm = await screen.findByRole("button", { name: "Delete" });
    await waitFor(() => expect(document.activeElement).toBe(confirm));
  });

  it("escape_cancels", async () => {
    const user = userEvent.setup();
    const { onConfirm, onCancel } = show();
    await screen.findByRole("alertdialog");
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("confirm_calls_once", async () => {
    const user = userEvent.setup();
    const { onConfirm, onCancel } = show();
    await user.click(await screen.findByRole("button", { name: "Delete" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("cancel_button_cancels", async () => {
    const user = userEvent.setup();
    const { onConfirm, onCancel } = show();
    await user.click(await screen.findByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    const user = userEvent.setup();
    const { onConfirm } = show({ danger: true });
    const cancel = await screen.findByRole("button", { name: "Cancel" });
    await waitFor(() => expect(document.activeElement).toBe(cancel));
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Delete" }));
    await user.keyboard("{Enter}");
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("has_no_accessibility_violations", async () => {
    show({ danger: true });
    const dialog = await screen.findByRole("alertdialog");
    await expectNoAccessibilityViolations(dialog.parentElement ?? dialog, { landmarks: false });
  });

  it("a_third_choice_sits_between_cancel_and_confirm_and_calls_its_own_function", async () => {
    const user = userEvent.setup();
    const onAlternate = vi.fn();
    const { onConfirm, onCancel } = show({ alternateLabel: "Discard", onAlternate, confirmLabel: "Save", title: "Unsaved changes" });
    await screen.findByRole("alertdialog", { name: "Unsaved changes" });
    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Cancel", "Discard", "Save"]);
    await user.click(screen.getByRole("button", { name: "Discard" }));
    expect(onAlternate).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
  });
});
