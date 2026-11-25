import { describe, expect, it } from "vitest";
import type { WireGuardView } from "./types";
import { checkLabel, checkWireguard, isCidr, isIp } from "./validate";

// 32 bytes of base64, the form of a WireGuard key
const KEY = "c2VlZC1wdWJsaWMta2V5LWZvci10aGUtdGVzdHMtMDA=";

const good = (): WireGuardView => ({
  addresses: ["10.8.0.2/32", "fd00::2/128"],
  dns: ["10.8.0.1", "fd00::1"],
  search_domains: ["corp.example", "~lab.example"],
  mtu: 1380,
  peers: [{ public_key: KEY, endpoint: "vpn.example:51820", allowed_ips: ["0.0.0.0/0", "::/0"], persistent_keepalive: 25 }],
});
const withChange = (change: (p: WireGuardView) => void): WireGuardView => {
  const p = good();
  change(p);
  return p;
};

describe("checkLabel", () => {
  it.each([
    ["", "required"],
    ["a".repeat(65), "too_long"],
    ["one\ntwo", "invalid_chars"],
    ["bell\u0007", "invalid_chars"],
  ])("refuses_%j", (value, code) => expect(checkLabel(value, 64)).toBe(code));

  it("takes_the_longest_name_and_counts_characters_not_bytes", () => {
    expect(checkLabel("a".repeat(64), 64)).toBeNull();
    expect(checkLabel("è".repeat(64), 64)).toBeNull();
    expect(checkLabel("😀".repeat(64), 64)).toBeNull();
  });
});

describe("addresses", () => {
  it.each(["10.0.0.1", "0.0.0.0", "255.255.255.255", "::", "::1", "fd00::1", "2001:db8:0:0:0:0:0:1", "::ffff:10.0.0.1", "1:2:3:4:5:6:7::"])("%s_is_an_ip", (ip) => expect(isIp(ip)).toBe(true));
  it.each(["", "10.0.0", "10.0.0.256", "10.0.0.01", "dns.example", "1:2:3:4:5:6:7:8:9", "1::2::3", "12345::1", "1.2.3.4::", "fe80::1%eth0", "10.0.0.1/24"])("%j_is_not_an_ip", (ip) => expect(isIp(ip)).toBe(false));
  it.each(["10.8.0.2/32", "0.0.0.0/0", "::/0", "fd00::2/128"])("%s_is_a_network", (net) => expect(isCidr(net)).toBe(true));
  it.each(["10.8.0.2", "10.8.0.2/33", "::/129", "10.8.0.2/", "10.8.0.2/-1", "nope/24", "10.8.0.2/2x"])("%j_is_not_a_network", (net) => expect(isCidr(net)).toBe(false));
});

describe("checkWireguard", () => {
  const fields = (p: WireGuardView) => checkWireguard(p).map((i) => `${i.field}:${i.code}`);

  it("accepts_a_good_profile_and_the_optional_parts_missing", () => {
    expect(checkWireguard(good())).toEqual([]);
    expect(checkWireguard(withChange((p) => { p.dns = []; p.search_domains = []; p.mtu = null; p.peers[0].endpoint = null; p.peers[0].persistent_keepalive = null; }))).toEqual([]);
    expect(checkWireguard(withChange((p) => { p.mtu = 576; p.search_domains = ["~."]; p.peers[0].endpoint = "[2001:db8::1]:51820"; }))).toEqual([]);
  });

  it.each([
    ["address_without_prefix", (p: WireGuardView) => { p.addresses = ["10.8.0.2"]; }, "addresses:invalid_cidr"],
    ["no_address", (p: WireGuardView) => { p.addresses = []; }, "addresses:required"],
    ["dns_is_a_name", (p: WireGuardView) => { p.dns = ["dns.example"]; }, "dns:invalid_ip"],
    ["domain_with_a_space", (p: WireGuardView) => { p.search_domains = ["bad domain"]; }, "search_domains:invalid_domain"],
    ["domain_with_an_empty_label", (p: WireGuardView) => { p.search_domains = ["a..b"]; }, "search_domains:invalid_domain"],
    ["mtu_too_low", (p: WireGuardView) => { p.mtu = 575; }, "mtu:out_of_range"],
    ["mtu_too_high", (p: WireGuardView) => { p.mtu = 9001; }, "mtu:out_of_range"],
    ["no_peer", (p: WireGuardView) => { p.peers = []; }, "peers:required"],
    ["key_too_short", (p: WireGuardView) => { p.peers[0].public_key = "c2hvcnQ="; }, "peers.0.public_key:invalid_key"],
    ["key_not_base64", (p: WireGuardView) => { p.peers[0].public_key = "!!!"; }, "peers.0.public_key:invalid_key"],
    ["key_without_padding", (p: WireGuardView) => { p.peers[0].public_key = KEY.replace("=", ""); }, "peers.0.public_key:invalid_key"],
    ["endpoint_without_port", (p: WireGuardView) => { p.peers[0].endpoint = "vpn.example"; }, "peers.0.endpoint:invalid_endpoint"],
    ["endpoint_port_zero", (p: WireGuardView) => { p.peers[0].endpoint = "vpn.example:0"; }, "peers.0.endpoint:invalid_endpoint"],
    ["endpoint_port_too_big", (p: WireGuardView) => { p.peers[0].endpoint = "vpn.example:70000"; }, "peers.0.endpoint:invalid_endpoint"],
    ["endpoint_with_a_bad_label", (p: WireGuardView) => { p.peers[0].endpoint = "-a.example:51820"; }, "peers.0.endpoint:invalid_endpoint"],
    ["route_not_a_network", (p: WireGuardView) => { p.peers[0].allowed_ips = ["0.0.0.0"]; }, "peers.0.allowed_ips:invalid_cidr"],
    ["no_route", (p: WireGuardView) => { p.peers[0].allowed_ips = []; }, "peers.0.allowed_ips:required"],
    ["keepalive_too_big", (p: WireGuardView) => { p.peers[0].persistent_keepalive = 65536; }, "peers.0.persistent_keepalive:out_of_range"],
  ])("%s", (_name, change, expected) => expect(fields(withChange(change))).toEqual([expected]));

  it("reports_every_wrong_field_in_the_order_of_the_form", () => {
    expect(fields(withChange((p) => { p.mtu = 100; p.peers[0].endpoint = "x"; p.peers[0].public_key = "k"; p.addresses = []; }))).toEqual([
      "addresses:required",
      "mtu:out_of_range",
      "peers.0.public_key:invalid_key",
      "peers.0.endpoint:invalid_endpoint",
    ]);
  });

  it("a_number_that_is_not_a_whole_number_is_refused", () => {
    expect(fields(withChange((p) => { p.mtu = 1380.5; }))).toEqual(["mtu:invalid_number"]);
    expect(fields(withChange((p) => { p.peers[0].persistent_keepalive = Number.NaN; }))).toEqual(["peers.0.persistent_keepalive:invalid_number"]);
  });
});
