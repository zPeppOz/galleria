import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { Button } from "./Button";
import { icons } from "./icons";

describe("Button", () => {
  it("is_a_native_button_of_type_button", () => {
    render(<Button>Save</Button>);
    const button = screen.getByRole("button", { name: "Save" });
    expect(button.tagName).toBe("BUTTON");
    expect(button.getAttribute("type")).toBe("button");
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = render(
      <main>
        <Button>Save</Button>
        <Button variant="primary" size="lg" icon={icons.check}>Connect</Button>
        <Button disabled>Off</Button>
        <Button busy>Working</Button>
        <Button icon={icons.more} aria-label="More" variant="ghost" />
      </main>,
    );
    await expectNoAccessibilityViolations(container);
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    render(<Button onClick={onClick}>Save</Button>);
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Save" }));
    await user.keyboard("{Enter}");
    await user.keyboard(" ");
    expect(onClick).toHaveBeenCalledTimes(2);
  });

  it("disabled_and_busy_do_not_fire_click", async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    render(
      <>
        <Button disabled onClick={onClick}>Disabled</Button>
        <Button busy onClick={onClick}>Busy</Button>
      </>,
    );
    await user.click(screen.getByRole("button", { name: "Disabled" }));
    await user.click(screen.getByRole("button", { name: "Busy" }));
    expect(onClick).not.toHaveBeenCalled();
  });

  it("busy_is_announced", () => {
    render(<Button busy>Working</Button>);
    const button = screen.getByRole("button", { name: "Working" });
    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(document.activeElement === button || button.tabIndex === 0).toBe(true); // it stays in the tab order
  });

  it("icon_only_button_needs_a_label", () => {
    // The type refuses an icon-only button without a name (tsc runs on the tests: `pnpm typecheck`).
    // @ts-expect-error neither text nor aria-label
    const nameless = <Button icon={icons.more} />;
    expect(nameless).toBeTruthy();
    render(<Button icon={icons.more} aria-label="More" />);
    expect(screen.getByRole("button", { name: "More" })).toBeTruthy();
  });
});
