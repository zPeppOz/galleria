import { describe, expect, it } from "vitest";
import { build, fieldTab, fieldOrder, isDirty, toDraft } from "./draft";
import { makeDetail, SAMPLE_PROFILES } from "../test/fixtures";
import type { ProfileDetail } from "./types";

const KEY = "c2VlZC1wdWJsaWMta2V5LWZvci10aGUtdGVzdHMtMDA=";
const [OFFICE, , HOMELAB] = SAMPLE_PROFILES;
const wireguard = (): ProfileDetail => {
  const detail = makeDetail(HOMELAB);
  return { ...detail, wireguard: { ...detail.wireguard!, peers: [{ public_key: KEY, endpoint: "casa.example:51820", allowed_ips: ["0.0.0.0/0"], persistent_keepalive: 25 }] } };
};

describe("draft", () => {
  it("a_profile_as_it_is_has_no_changes_and_asks_for_nothing", () => {
    const detail = wireguard();
    expect(isDirty(toDraft(detail), detail)).toBe(false);
    expect(build(toDraft(detail), detail)).toEqual({ patch: {}, issues: [] });
  });

  it("lists_are_text_with_commas_and_numbers_are_text", () => {
    const draft = toDraft(wireguard());
    expect(draft.wireguard).toEqual({
      addresses: "10.8.0.2/32",
      dns: "",
      search_domains: "",
      mtu: "",
      peers: [{ public_key: KEY, endpoint: "casa.example:51820", allowed_ips: "0.0.0.0/0", persistent_keepalive: "25" }],
    });
    expect(toDraft(makeDetail(OFFICE)).wireguard).toBeNull();
  });

  it("only_what_changed_goes_in_the_patch", () => {
    const detail = wireguard();
    const draft = toDraft(detail);
    draft.name = "  Casa  ";
    draft.notes = "dietro il router";
    draft.group = "";
    expect(isDirty(draft, detail)).toBe(true);
    expect(build(draft, detail)).toEqual({ patch: { name: "Casa", notes: "dietro il router", group: null }, issues: [] });
  });

  it("the_wireguard_text_is_turned_into_the_typed_view", () => {
    const detail = wireguard();
    const draft = toDraft(detail);
    draft.wireguard!.dns = "10.8.0.1, fd00::1";
    draft.wireguard!.search_domains = "corp.example  lab.example";
    draft.wireguard!.mtu = "1380";
    draft.wireguard!.peers[0].endpoint = "";
    draft.wireguard!.peers[0].persistent_keepalive = "";
    draft.wireguard!.peers[0].allowed_ips = "10.0.0.0/8,192.168.0.0/16";
    expect(build(draft, detail)).toEqual({
      patch: {
        wireguard: {
          addresses: ["10.8.0.2/32"],
          dns: ["10.8.0.1", "fd00::1"],
          search_domains: ["corp.example", "lab.example"],
          mtu: 1380,
          peers: [{ public_key: KEY, endpoint: null, allowed_ips: ["10.0.0.0/8", "192.168.0.0/16"], persistent_keepalive: null }],
        },
      },
      issues: [],
    });
  });

  it("wrong_values_come_back_as_issues_with_the_path_and_no_patch_for_them", () => {
    const detail = wireguard();
    const draft = toDraft(detail);
    draft.name = "";
    draft.wireguard!.mtu = "abc";
    draft.wireguard!.peers[0].public_key = "short";
    const { patch, issues } = build(draft, detail);
    expect(issues).toEqual([
      { field: "name", code: "required" },
      { field: "mtu", code: "invalid_number" },
      { field: "peers.0.public_key", code: "invalid_key" },
    ]);
    expect(patch.name).toBeUndefined();
  });

  it("the_fields_are_ordered_as_the_tabs_show_them", () => {
    expect(fieldOrder(2)).toEqual([
      "name", "group", "notes", "peers.0.endpoint", "peers.1.endpoint",
      "peers.0.public_key", "peers.1.public_key",
      "addresses", "dns", "search_domains", "mtu", "peers.0.allowed_ips", "peers.0.persistent_keepalive", "peers.1.allowed_ips", "peers.1.persistent_keepalive",
    ]);
    expect(fieldTab("name")).toBe("general");
    expect(fieldTab("peers.3.endpoint")).toBe("general");
    expect(fieldTab("peers.0.public_key")).toBe("auth");
    expect(fieldTab("mtu")).toBe("network");
    expect(fieldTab("peers.1.allowed_ips")).toBe("network");
  });
});
