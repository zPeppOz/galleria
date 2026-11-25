import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { Dialog } from "./Dialog";

function Harness({ description = "The version of the app." }: { description?: string }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>Open</button>
      <button type="button">Behind</button>
      <Dialog open={open} onClose={() => setOpen(false)} title="About Galleria" description={description} closeLabel="Close" footer={<button type="button">Footer action</button>}>
        <p>Body text</p>
        <a href="#licence">Licence</a>
      </Dialog>
    </>
  );
}

async function openIt() {
  const user = userEvent.setup();
  render(<Harness />);
  await user.click(screen.getByRole("button", { name: "Open" }));
  return { user, dialog: await screen.findByRole("dialog") };
}

describe("Dialog", () => {
  it("dialog_opens_with_focus_inside_and_is_named_by_its_title", async () => {
    const { dialog } = await openIt();
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    expect(screen.getByRole("dialog", { name: "About Galleria" })).toBe(dialog);
    await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));
  });

  it("tab_cycles_inside_the_dialog", async () => {
    const { user, dialog } = await openIt();
    await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));
    // The library parks focus on an invisible guard at the edge for an instant and moves it back: wait for it.
    for (let i = 0; i < 8; i++) {
      await user.tab();
      await waitFor(() => expect(dialog.contains(document.activeElement), `after ${i + 1} Tab`).toBe(true));
    }
    for (let i = 0; i < 8; i++) {
      await user.tab({ shift: true });
      await waitFor(() => expect(dialog.contains(document.activeElement), `after ${i + 1} Shift+Tab`).toBe(true));
    }
    // What is behind the dialog is not reachable by assistive technology either.
    expect(screen.queryByRole("button", { name: "Behind" })).toBeNull();
  });

  it("dialog_is_described_by_its_description", async () => {
    const { dialog } = await openIt();
    const id = dialog.getAttribute("aria-describedby");
    expect(id).toBeTruthy();
    expect(document.getElementById(id ?? "")?.textContent).toBe("The version of the app.");
  });

  it("escape_closes_and_returns_focus_to_the_opener", async () => {
    const { user } = await openIt();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Open" }));
  });

  it("close_button_closes_and_returns_focus_to_the_opener", async () => {
    const { user } = await openIt();
    await user.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Open" }));
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    render(<Harness />);
    const user = userEvent.setup();
    await user.tab();
    await user.keyboard("{Enter}");
    expect(await screen.findByRole("dialog")).toBeTruthy();
    await user.tab();
    await user.keyboard("{Enter}");
  });

  it("has_no_accessibility_violations", async () => {
    await openIt();
    await expectNoAccessibilityViolations(document.body, { landmarks: false });
  });
});
