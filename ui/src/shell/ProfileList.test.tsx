import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { I18n } from "../i18n";
import { expectNoAccessibilityViolations } from "../test/axe";
import { ProfileList, type ListGroup } from "./ProfileList";

const GROUPS: ListGroup[] = [
  {
    id: "work",
    title: "Work",
    profiles: [
      { id: "a", name: "Alpha", server: "a.example:1194", protocol: "openvpn", favourite: true },
      { id: "b", name: "Bravo", server: "b.example:51820", protocol: "wireguard" },
      { id: "c", name: "Bravo Two", server: "c.example:51820", protocol: "wireguard" },
    ],
  },
  {
    id: "home",
    title: "Home",
    profiles: [
      { id: "d", name: "Delta", server: "d.example:1194", protocol: "openvpn" },
      { id: "e", name: "Echo", server: "e.example:51820", protocol: "wireguard" },
    ],
  },
];

function Harness({ initial = "a" }: { initial?: string }) {
  const [selected, setSelected] = useState(initial);
  return (
    <I18n lang="en">
      <button type="button">before</button>
      <ProfileList label="Connections" groups={GROUPS} selectedId={selected} onSelect={setSelected} />
      <button type="button">after</button>
    </I18n>
  );
}

const option = (name: string) => screen.getByRole("option", { name });
const selectedName = () => screen.getAllByRole("option").filter((o) => o.getAttribute("aria-selected") === "true").map((o) => o.textContent);
const description = (el: HTMLElement) =>
  (el.getAttribute("aria-describedby") ?? "").split(" ").map((id) => document.getElementById(id)?.textContent ?? "").join(" ");

describe("ProfileList", () => {
  it("is_a_listbox_with_named_groups_and_options", () => {
    render(<Harness />);
    expect(screen.getByRole("listbox", { name: "Connections" })).toBeTruthy();
    expect(screen.getAllByRole("group").map((g) => g.getAttribute("aria-label"))).toEqual(["Work", "Home"]);
    expect(screen.getAllByRole("option").map((o) => o.getAttribute("aria-labelledby") && document.getElementById(o.getAttribute("aria-labelledby") ?? "")?.textContent)).toEqual([
      "Alpha", "Bravo", "Bravo Two", "Delta", "Echo",
    ]);
    expect(option("Alpha")).toBeTruthy();
  });

  it("selected_option_is_exposed", () => {
    render(<Harness initial="d" />);
    expect(screen.getAllByRole("option").filter((o) => o.getAttribute("aria-selected") === "true")).toHaveLength(1);
    expect(option("Delta").getAttribute("aria-selected")).toBe("true");
  });

  it("protocol_and_favourite_have_a_text_alternative", () => {
    render(<Harness />);
    expect(description(option("Alpha"))).toContain("OpenVPN");
    expect(description(option("Alpha"))).toContain("Favourite");
    expect(description(option("Alpha"))).toContain("a.example:1194");
    expect(description(option("Bravo"))).toContain("WireGuard");
    expect(description(option("Bravo"))).not.toContain("Favourite");
  });

  it("is_one_tab_stop", async () => {
    const user = userEvent.setup();
    render(<Harness initial="c" />);
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "before" }));
    await user.tab();
    expect(document.activeElement).toBe(option("Bravo Two"));
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "after" }));
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(option("Bravo Two"));
  });

  it("arrows_home_end_move_and_select", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab();
    expect(document.activeElement).toBe(option("Alpha"));
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(option("Bravo"));
    expect(selectedName()[0]).toContain("Bravo");
    await user.keyboard("{ArrowDown}{ArrowDown}"); // crosses into the next group
    expect(document.activeElement).toBe(option("Delta"));
    expect(option("Delta").getAttribute("aria-selected")).toBe("true");
    await user.keyboard("{End}");
    expect(document.activeElement).toBe(option("Echo"));
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(option("Echo"));
    await user.keyboard("{Home}");
    expect(document.activeElement).toBe(option("Alpha"));
    await user.keyboard("{ArrowUp}");
    expect(document.activeElement).toBe(option("Alpha"));
    expect(selectedName()).toHaveLength(1);
  });

  it("typing_a_letter_jumps_to_the_next_match", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.tab();
    await user.tab();
    await user.keyboard("b");
    expect(document.activeElement).toBe(option("Bravo"));
    await user.keyboard("b");
    expect(document.activeElement).toBe(option("Bravo Two"));
    await user.keyboard("b");
    expect(document.activeElement).toBe(option("Bravo"));
    await user.keyboard("e");
    expect(document.activeElement).toBe(option("Echo"));
    expect(option("Echo").getAttribute("aria-selected")).toBe("true");
  });

  it("a_click_selects", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(option("Delta"));
    expect(option("Delta").getAttribute("aria-selected")).toBe("true");
  });

  it("has_no_accessibility_violations", async () => {
    const { container } = render(<main><Harness /></main>);
    await expectNoAccessibilityViolations(container);
  });
});

describe("ProfileList: the session and the search", () => {
  const withState: ListGroup[] = [
    { id: "g", title: "Work", profiles: [{ id: "a", name: "Alpha", server: "a.example:1194", protocol: "openvpn", state: "connected" }, { id: "b", name: "Bravo", server: "b.example:51820", protocol: "wireguard" }] },
  ];
  const show = (groups: ListGroup[], empty?: string) =>
    render(
      <I18n lang="en">
        <ProfileList label="Connections" groups={groups} selectedId="b" onSelect={() => undefined} empty={empty} />
      </I18n>,
    );

  it("active_session_row_says_its_state_in_words", () => {
    show(withState);
    // the state is a text on the row and a part of the name: not only the colour of the dot
    const row = screen.getByRole("option", { name: "Alpha Connected" });
    expect(row.textContent).toContain("Connected");
    expect(row.querySelector(".profile-dot")?.getAttribute("aria-hidden")).toBe("true");
    expect(row.querySelector(".profile-dot")?.className).toContain("profile-dot-connected");
  });

  it.each([["connecting", "Connecting"], ["reconnecting", "Reconnecting"], ["needs_input", "Waiting for input"], ["error", "Error"]] as const)("every_state_has_its_word: %s", (state, word) => {
    show([{ id: "g", title: "Work", profiles: [{ ...withState[0].profiles[0], state }] }]);
    expect(screen.getByRole("option", { name: `Alpha ${word}` })).toBeTruthy();
  });

  it("a_row_without_a_session_has_no_state", () => {
    show(withState);
    const row = screen.getByRole("option", { name: "Bravo" });
    expect(row.querySelector(".profile-state")).toBeNull();
    expect(row.querySelector(".profile-dot")?.className).toBe("profile-dot");
  });

  it("no_match_says_so", async () => {
    const { container } = render(
      <I18n lang="en">
        <main>
          <ProfileList label="Connections" groups={[]} selectedId="" onSelect={() => undefined} empty="No connection matches your search." />
        </main>
      </I18n>,
    );
    // the listbox stays in the page, with no option, and the text says why
    expect(screen.getByRole("listbox", { name: "Connections" })).toBeTruthy();
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByText("No connection matches your search.")).toBeTruthy();
    await expectNoAccessibilityViolations(container);
  });

  it("the_empty_text_is_not_shown_while_there_are_profiles", () => {
    show(withState, "No connection matches your search.");
    expect(screen.queryByText("No connection matches your search.")).toBeNull();
  });

  it("the_focus_ref_hands_the_focus_to_the_chosen_row", () => {
    const ref = { current: null as { focus: () => void } | null };
    render(
      <I18n lang="en">
        <ProfileList label="Connections" groups={withState} selectedId="b" onSelect={() => undefined} focusRef={ref} />
      </I18n>,
    );
    ref.current?.focus();
    expect(document.activeElement).toBe(screen.getByRole("option", { name: "Bravo" }));
  });
});
