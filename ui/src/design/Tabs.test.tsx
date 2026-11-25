import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { Tabs } from "./Tabs";

const TABS = [
  { id: "general", label: "General", panel: <button type="button">in general</button> },
  { id: "network", label: "Network", panel: <p>network panel</p> },
  { id: "advanced", label: "Advanced", panel: <p>advanced panel</p> },
];

function Harness() {
  const [value, setValue] = useState("general");
  return (
    <>
      <button type="button">before</button>
      <Tabs label="Profile" value={value} onChange={setValue} tabs={TABS} />
    </>
  );
}

describe("Tabs", () => {
  it("has_no_accessibility_violations", async () => {
    const { container } = render(<main><Harness /></main>);
    await expectNoAccessibilityViolations(container);
  });

  it("has_tablist_tab_and_tabpanel_roles_linked_together", () => {
    render(<Harness />);
    expect(screen.getByRole("tablist", { name: "Profile" })).toBeTruthy();
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.textContent)).toEqual(["General", "Network", "Advanced"]);
    expect(tabs.map((t) => t.getAttribute("aria-selected"))).toEqual(["true", "false", "false"]);
    const panel = screen.getByRole("tabpanel");
    expect(panel.getAttribute("aria-labelledby")).toBe(tabs[0].id);
    expect(tabs[0].getAttribute("aria-controls")).toBe(panel.id);
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "General" }));
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Network" }).getAttribute("aria-selected")).toBe("true");
  });

  it("arrows_home_and_end_move_between_tabs", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab();
    await user.keyboard("{ArrowRight}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Network" }));
    await user.keyboard("{End}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Advanced" }));
    await user.keyboard("{Home}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "General" }));
    await user.keyboard("{ArrowLeft}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Advanced" })); // wraps
  });

  it("only_the_selected_panel_is_rendered", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    expect(screen.queryByText("network panel")).toBeNull();
    await user.click(screen.getByRole("tab", { name: "Network" }));
    expect(screen.getByText("network panel")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "in general" })).toBeNull();
  });

  it("tablist_is_one_tab_stop", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab(); // the selected tab
    await user.tab(); // out of the list: the panel (it takes the focus itself), not the next tab
    expect(document.activeElement).toBe(screen.getByRole("tabpanel"));
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "in general" }));
    expect(screen.getByRole("tab", { name: "Network" }).tabIndex).toBe(-1);
  });
});
