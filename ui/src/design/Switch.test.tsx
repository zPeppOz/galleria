import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { Switch } from "./Switch";

function Harness(props: { initial?: boolean; disabled?: boolean; hint?: string }) {
  const [checked, setChecked] = useState(props.initial ?? false);
  return <Switch label="Start at login" checked={checked} disabled={props.disabled} hint={props.hint} onChange={setChecked} />;
}

describe("Switch", () => {
  it("has_no_accessibility_violations", async () => {
    const { container } = render(
      <main>
        <Harness hint="Opens the window when you sign in" />
        <Harness initial disabled />
      </main>,
    );
    await expectNoAccessibilityViolations(container);
  });

  it("has_switch_role_and_checked_state", async () => {
    const user = userEvent.setup();
    render(<Harness hint="Opens the window" />);
    const toggle = screen.getByRole("switch", { name: "Start at login" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(toggle.getAttribute("aria-describedby")).toBeTruthy();
    await user.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("true");
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("switch"));
  });

  it("space_and_enter_toggle", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const toggle = screen.getByRole("switch");
    await user.tab();
    await user.keyboard(" ");
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    await user.keyboard("{Enter}");
    expect(toggle.getAttribute("aria-checked")).toBe("false");
  });

  it("disabled_switch_does_not_toggle", async () => {
    const user = userEvent.setup();
    render(<Harness disabled />);
    const toggle = screen.getByRole("switch");
    await user.click(toggle);
    await user.click(screen.getByText("Start at login"));
    expect(toggle.getAttribute("aria-checked")).toBe("false");
  });

  it("label_click_toggles", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByText("Start at login"));
    expect(screen.getByRole("switch").getAttribute("aria-checked")).toBe("true");
  });
});
