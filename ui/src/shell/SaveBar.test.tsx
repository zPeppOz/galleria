import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { SaveBar } from "./SaveBar";

const show = (over: Partial<Parameters<typeof SaveBar>[0]> = {}) => {
  const onSave = vi.fn();
  const onCancel = vi.fn();
  const view = render(
    <I18n lang="en">
      <main>
        <SaveBar dirty busy={false} onSave={onSave} onCancel={onCancel} {...over} />
      </main>
    </I18n>,
  );
  return { onSave, onCancel, ...view };
};

describe("SaveBar", () => {
  it("appears_only_when_dirty", () => {
    const { rerender, onSave, onCancel } = show({ dirty: false });
    expect(screen.queryByRole("group", { name: "Unsaved changes" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();
    rerender(
      <I18n lang="en">
        <main>
          <SaveBar dirty busy={false} onSave={onSave} onCancel={onCancel} />
        </main>
      </I18n>,
    );
    expect(screen.getByRole("group", { name: "Unsaved changes" })).toBeTruthy();
    expect(screen.getByText("You have changes that are not saved.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeTruthy();
  });

  it("ctrl_s_saves_from_anywhere_and_the_browser_does_not_get_it", async () => {
    const user = userEvent.setup();
    const { onSave } = show();
    const event = new KeyboardEvent("keydown", { key: "s", ctrlKey: true, bubbles: true, cancelable: true });
    document.body.dispatchEvent(event);
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(event.defaultPrevented).toBe(true);
    await user.keyboard("{Control>}s{/Control}");
    expect(onSave).toHaveBeenCalledTimes(2);
  });

  it("ctrl_s_does_nothing_when_there_is_nothing_to_save_or_a_save_is_going", () => {
    const idle = show({ dirty: false });
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "s", ctrlKey: true, bubbles: true }));
    expect(idle.onSave).not.toHaveBeenCalled();
    idle.unmount();
    const busy = show({ busy: true });
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "s", ctrlKey: true, bubbles: true }));
    expect(busy.onSave).not.toHaveBeenCalled();
  });

  it("cancel_restores_the_values", async () => {
    const user = userEvent.setup();
    const { onCancel, onSave } = show();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onSave).not.toHaveBeenCalled();
  });

  it("a_save_that_is_going_says_so_and_cannot_be_clicked_again", async () => {
    const user = userEvent.setup();
    const { onSave } = show({ busy: true });
    const save = screen.getByRole("button", { name: "Save" });
    expect(save.getAttribute("aria-busy")).toBe("true");
    await user.click(save);
    expect(onSave).not.toHaveBeenCalled();
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = show();
    await expectNoAccessibilityViolations(container);
  });
});
