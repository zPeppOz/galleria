import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { expectNoAccessibilityViolations } from "../test/axe";
import { Menu, type MenuEntry } from "./Menu";

function Harness({ onPick = () => {}, onLang = () => {} }: { onPick?: () => void; onLang?: (v: string) => void }) {
  const [lang, setLang] = useState("system");
  const entries: MenuEntry[] = [
    { kind: "radio", id: "lang", label: "Language", value: lang, options: [{ value: "system", label: "System" }, { value: "en", label: "English" }, { value: "it", label: "Italiano" }], onChange: (v) => { setLang(v); onLang(v); } },
    { kind: "separator" },
    { kind: "item", id: "about", label: "About Galleria", onSelect: onPick },
    { kind: "item", id: "bye", label: "Zeta", onSelect: onPick },
  ];
  return <Menu label="Main menu" trigger="Menu" entries={entries} />;
}

const trigger = () => screen.getByRole("button", { name: "Menu" });

describe("Menu", () => {
  it("trigger_exposes_haspopup_and_expanded", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    expect(trigger().getAttribute("aria-haspopup")).toBe("menu");
    expect(trigger().getAttribute("aria-expanded")).toBe("false");
    await user.click(trigger());
    await screen.findByRole("menu");
    expect(trigger().getAttribute("aria-expanded")).toBe("true");
  });

  it.each([["{Enter}"], [" "], ["{ArrowDown}"]])("menu_opens_from_the_keyboard_on_the_first_item: %s", async (key) => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    expect(document.activeElement).toBe(trigger());
    await user.keyboard(key);
    const menu = await screen.findByRole("menu", { name: "Main menu" });
    expect(menu).toBeTruthy();
    const first = screen.getByRole("menuitemradio", { name: "System" });
    await waitFor(() => expect(document.activeElement).toBe(first));
  });

  it("arrow_home_end_and_typeahead_move_the_focus", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    const focused = () => document.activeElement?.textContent;
    await waitFor(() => expect(focused()).toBe("System"));
    await user.keyboard("{ArrowDown}");
    expect(focused()).toBe("English");
    await user.keyboard("{ArrowDown}{ArrowDown}");
    expect(focused()).toBe("About Galleria");
    await user.keyboard("{ArrowUp}");
    expect(focused()).toBe("Italiano");
    await user.keyboard("{End}");
    expect(focused()).toBe("Zeta");
    await user.keyboard("{Home}");
    expect(focused()).toBe("System");
    await user.keyboard("a");
    expect(focused()).toBe("About Galleria");
  });

  it("radio_items_expose_and_change_their_checked_state", async () => {
    const user = userEvent.setup();
    const onLang = vi.fn();
    render(<Harness onLang={onLang} />);
    await user.tab();
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    const checked = () => screen.getAllByRole("menuitemradio").filter((el) => el.getAttribute("aria-checked") === "true").map((el) => el.textContent);
    expect(checked()).toEqual(["System"]);
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    expect(onLang).toHaveBeenCalledWith("it");
    // choosing an item closes the menu; reopen it to read the state
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    expect(checked()).toEqual(["Italiano"]);
  });

  it("escape_closes_the_menu_and_returns_focus", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(document.activeElement).toBe(trigger());
    expect(trigger().getAttribute("aria-expanded")).toBe("false");
  });

  it("choosing_an_item_runs_it_and_closes_the_menu", async () => {
    const user = userEvent.setup();
    const onPick = vi.fn();
    render(<Harness onPick={onPick} />);
    await user.tab();
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    await waitFor(() => expect(document.activeElement?.textContent).toBe("System"));
    await user.keyboard("{End}{Enter}");
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(onPick).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(trigger());
  });

  it("is_reachable_and_operable_from_the_keyboard", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    expect(document.activeElement).toBe(trigger());
  });

  it("has_no_accessibility_violations", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(trigger());
    await screen.findByRole("menu");
    await expectNoAccessibilityViolations(document.body, { landmarks: false });
  });
});
