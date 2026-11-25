import { describe, expect, it } from "vitest";
import { makeSummary } from "../test/fixtures";
import { arrange } from "./arrange";
import type { ProfileSummary } from "./types";

const profile = (name: string, group: string | null, over: Partial<ProfileSummary> = {}) => makeSummary({ id: name, name, group, ...over });
const names = (arranged: ReturnType<typeof arrange>) => arranged.groups.flatMap((g) => g.profiles.map((p) => p.name));
const base = { query: "", sort: "name" as const };

describe("arrange", () => {
  it("groups_follow_the_saved_order_then_the_unknown_ones_then_no_group", () => {
    const arranged = arrange({
      ...base,
      groups: ["Zeta", "Alfa"],
      profiles: [profile("a", null), profile("b", "Beta"), profile("c", "Alfa"), profile("d", "Zeta"), profile("e", "Aaa"), profile("f", "Zeta")],
    });
    // the saved order first (not the alphabetical one), then the groups that only the profiles know, then the rest
    expect(arranged.groups.map((g) => g.title)).toEqual(["Zeta", "Alfa", "Aaa", "Beta", null]);
    expect(arranged.groups[0].profiles.map((p) => p.name)).toEqual(["d", "f"]);
    expect(arranged.total).toBe(6);
  });

  it("an_empty_rubrica_has_no_group_at_all", () => {
    expect(arrange({ ...base, groups: [], profiles: [] })).toEqual({ groups: [], total: 0 });
  });

  it.each(["name", "recent"] as const)("favourites_come_first_inside_their_group: %s", (sort) => {
    const arranged = arrange({
      query: "",
      sort,
      groups: ["Lavoro", "Casa"],
      profiles: [
        profile("Alfa", "Lavoro"),
        profile("Zulu", "Lavoro", { favorite: true }),
        profile("Beta", "Casa", { favorite: true }),
        profile("Aaa", "Casa"),
        // a favourite of another group does not jump over the groups
        profile("Yankee", "Casa", { favorite: true }),
      ],
    });
    expect(arranged.groups.map((g) => g.profiles.map((p) => p.name))).toEqual([["Zulu", "Alfa"], ["Beta", "Yankee", "Aaa"]]);
  });

  it("the_name_order_is_by_name_and_then_by_id", () => {
    const arranged = arrange({ ...base, groups: [], profiles: [profile("b", null, { id: "2" }), profile("b", null, { id: "1" }), profile("a", null, { id: "3" })] });
    expect(arranged.groups[0].profiles.map((p) => p.id)).toEqual(["3", "1", "2"]);
  });

  it("recent_order_uses_last_used_and_falls_back_to_the_name", () => {
    const arranged = arrange({
      query: "",
      sort: "recent",
      groups: [],
      profiles: [
        profile("Delta", null),
        profile("Alfa", null, { last_used: "2026-11-01T10:00:00Z" }),
        profile("Charlie", null, { last_used: "2026-11-20T10:00:00Z" }),
        profile("Bravo", null),
        profile("Echo", null, { last_used: "not a date" }),
      ],
    });
    // the used ones from the latest to the oldest, then the rest by name (a value that is not a date counts as never used)
    expect(names(arranged)).toEqual(["Charlie", "Alfa", "Bravo", "Delta", "Echo"]);
  });

  it("recent_order_without_any_use_is_the_name_order", () => {
    const profiles = [profile("Zulu", null), profile("Alfa", null), profile("Mike", null)];
    expect(names(arrange({ query: "", sort: "recent", groups: [], profiles }))).toEqual(names(arrange({ ...base, groups: [], profiles })));
  });

  it("search_ignores_case_and_accents_and_needs_every_word", () => {
    const profiles = [profile("Città Milano", null), profile("Milano Nord", null), profile("Roma", null)];
    const found = (query: string) => names(arrange({ query, sort: "name", groups: [], profiles }));
    expect(found("citta mi")).toEqual(["Città Milano"]);
    expect(found("CITTÀ")).toEqual(["Città Milano"]);
    expect(found("milano")).toEqual(["Città Milano", "Milano Nord"]);
    expect(found("milano roma")).toEqual([]);
    expect(found("   milano   nord  ")).toEqual(["Milano Nord"]);
    expect(arrange({ query: "milano", sort: "name", groups: [], profiles }).total).toBe(2);
  });

  it("search_looks_at_name_server_and_group", () => {
    const profiles = [profile("Uno", "Lavoro", { server: "vpn.ufficio.example:1194" }), profile("Due", "Casa", { server: "casa.example:51820" }), profile("Tre", null, { server: null })];
    const found = (query: string) => names(arrange({ query, sort: "name", groups: [], profiles }));
    expect(found("ufficio")).toEqual(["Uno"]);
    expect(found("1194")).toEqual(["Uno"]);
    expect(found("casa")).toEqual(["Due"]);
    expect(found("lavoro")).toEqual(["Uno"]);
    expect(found("tre")).toEqual(["Tre"]);
    // a profile with no server is not found by the word «null»
    expect(found("null")).toEqual([]);
  });

  it("empty_groups_show_only_without_a_query", () => {
    const input = { sort: "name" as const, groups: ["Vuoto", "Pieno"], profiles: [profile("Alfa", "Pieno")] };
    expect(arrange({ ...input, query: "" }).groups.map((g) => [g.title, g.profiles.length])).toEqual([["Vuoto", 0], ["Pieno", 1]]);
    expect(arrange({ ...input, query: "alfa" }).groups.map((g) => [g.title, g.profiles.length])).toEqual([["Pieno", 1]]);
    // and a query that matches nothing leaves no group at all
    expect(arrange({ ...input, query: "zzz" })).toEqual({ groups: [], total: 0 });
  });

  it("names_sort_with_the_collator_of_the_language", () => {
    const profiles = [profile("Zebra", null), profile("Äpfel", null), profile("Apfel", null)];
    // German puts «Ä» with «A»; Swedish puts it after «Z»
    expect(names(arrange({ ...base, groups: [], profiles, locale: "de" }))).toEqual(["Apfel", "Äpfel", "Zebra"]);
    expect(names(arrange({ ...base, groups: [], profiles, locale: "sv" }))).toEqual(["Apfel", "Zebra", "Äpfel"]);
  });

  it("does_not_change_what_it_is_given", () => {
    const profiles = [profile("b", null), profile("a", null)];
    const copy = [...profiles];
    arrange({ ...base, groups: [], profiles });
    expect(profiles).toEqual(copy);
  });
});
