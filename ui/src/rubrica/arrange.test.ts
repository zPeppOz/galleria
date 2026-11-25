import { describe, expect, it } from "vitest";
import { makeSummary } from "../test/fixtures";
import { arrange } from "./arrange";

const profile = (name: string, group: string | null) => makeSummary({ id: name, name, group });

describe("arrange", () => {
  it("groups_follow_the_saved_order_then_the_unknown_ones_then_no_group", () => {
    const arranged = arrange({
      groups: ["Zeta", "Alfa"],
      profiles: [profile("a", null), profile("b", "Beta"), profile("c", "Alfa"), profile("d", "Zeta"), profile("e", "Aaa"), profile("f", "Zeta")],
    });
    // the saved order first (not the alphabetical one), then the groups that only the profiles know, then the rest
    expect(arranged.groups.map((g) => g.title)).toEqual(["Zeta", "Alfa", "Aaa", "Beta", null]);
    expect(arranged.groups[0].profiles.map((p) => p.name)).toEqual(["d", "f"]);
    expect(arranged.total).toBe(6);
  });

  it("a_group_of_the_list_without_profiles_is_still_shown", () => {
    const arranged = arrange({ groups: ["Vuoto", "Pieno"], profiles: [profile("a", "Pieno")] });
    expect(arranged.groups.map((g) => [g.title, g.profiles.length])).toEqual([["Vuoto", 0], ["Pieno", 1]]);
  });

  it("an_empty_rubrica_has_no_group_at_all", () => {
    expect(arrange({ groups: [], profiles: [] })).toEqual({ groups: [], total: 0 });
  });
});
