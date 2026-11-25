import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { Segmented } from "./Segmented";

const OPTIONS = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

function Harness(props: { readOnly?: boolean }) {
  const [value, setValue] = useState("system");
  return (
    <>
      <button type="button">before</button>
      <Segmented label="Theme" value={value} options={OPTIONS} onChange={setValue} readOnly={props.readOnly} />
      <button type="button">after</button>
    </>
  );
}

describe("Segmented", () => {
  it("has_no_accessibility_violations", async () => {
    const { container } = render(<main><Harness /></main>);
    await expectNoAccessibilityViolations(container);
  });

  it("is_a_radiogroup_with_one_checked_option", () => {
    render(<Harness />);
    expect(screen.getByRole("radiogroup", { name: "Theme" })).toBeTruthy();
    const radios = screen.getAllByRole("radio") as HTMLInputElement[];
    expect(radios.map((r) => r.checked)).toEqual([true, false, false]);
    expect(screen.getByRole("radio", { name: "System" })).toBeTruthy();
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("radio", { name: "System" }));
  });

  it("arrows_move_the_choice", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab();
    await user.keyboard("{ArrowRight}");
    expect((screen.getByRole("radio", { name: "Light" }) as HTMLInputElement).checked).toBe(true);
    expect(document.activeElement).toBe(screen.getByRole("radio", { name: "Light" }));
    await user.keyboard("{ArrowDown}");
    expect((screen.getByRole("radio", { name: "Dark" }) as HTMLInputElement).checked).toBe(true);
    await user.keyboard("{ArrowLeft}");
    expect((screen.getByRole("radio", { name: "Light" }) as HTMLInputElement).checked).toBe(true);
  });

  it("is_one_tab_stop", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab();
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "after" }));
  });

  it("read_only_shows_the_value_and_does_not_change", async () => {
    const user = userEvent.setup();
    render(<Harness readOnly />);
    await user.click(screen.getByText("Dark"));
    await user.tab();
    await user.tab();
    await user.keyboard("{ArrowRight}");
    expect((screen.getAllByRole("radio") as HTMLInputElement[]).map((r) => r.checked)).toEqual([true, false, false]);
    expect(screen.getByRole("radiogroup").getAttribute("aria-readonly")).toBe("true");
  });
});
