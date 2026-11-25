import type { FieldIssue, WireGuardView } from "./types";

// The same checks as `crates/galleria-gui/src/edit.rs`, so that a wrong value is told while it is typed. The authority is
// Rust: it checks again on save (and a test there compares its rules with the daemon's). Pure: nothing from React or Tauri.

export const NAME_MAX = 64;
export const GROUP_MAX = 48;

/** Why a label (the name of a profile or of a group) is not good, or null. Length is in characters, as in Rust. */
export function checkLabel(value: string, max: number): string | null {
  const length = [...value].length;
  if (length === 0) return "required";
  if (length > max) return "too_long";
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f-\u009f]/.test(value)) return "invalid_chars";
  return null;
}

const isIpv4 = (text: string): boolean => {
  const parts = text.split(".");
  return parts.length === 4 && parts.every((part) => /^(0|[1-9]\d{0,2})$/.test(part) && Number(part) <= 255);
};

/** How many 16-bit groups a run of an IPv6 address holds, or null when it is not one. An IPv4 tail counts for two and may only end the address. */
function groups(run: string, mayEndWithIpv4: boolean): number | null {
  if (run === "") return 0;
  const parts = run.split(":");
  let count = 0;
  for (const [index, part] of parts.entries()) {
    if (index === parts.length - 1 && part.includes(".")) {
      if (!mayEndWithIpv4 || !isIpv4(part)) return null;
      count += 2;
    } else if (/^[0-9a-fA-F]{1,4}$/.test(part)) {
      count += 1;
    } else {
      return null;
    }
  }
  return count;
}

const isIpv6 = (text: string): boolean => {
  if (!/^[0-9a-fA-F:.]+$/.test(text) || !text.includes(":")) return false;
  const halves = text.split("::");
  if (halves.length === 1) return groups(text, true) === 8;
  if (halves.length > 2) return false;
  const before = groups(halves[0], false);
  const after = groups(halves[1], true);
  return before !== null && after !== null && before + after <= 7;
};

export const isIp = (text: string): boolean => isIpv4(text) || isIpv6(text);

/** An address with its prefix: `10.8.0.2/32`, `::/0`. */
export function isCidr(text: string): boolean {
  const slash = text.indexOf("/");
  if (slash < 0) return false;
  const [address, prefix] = [text.slice(0, slash), text.slice(slash + 1)];
  if (!/^\d{1,3}$/.test(prefix)) return false;
  return (isIpv4(address) && Number(prefix) <= 32) || (isIpv6(address) && Number(prefix) <= 128);
}

const label = (part: string, extra: RegExp): boolean => part.length > 0 && part.length <= 63 && !part.startsWith("-") && !part.endsWith("-") && extra.test(part);

function isEndpoint(value: string): boolean {
  const bracket = /^\[(.+)\]:(\d+)$/.exec(value);
  if (bracket) return isIpv6(bracket[1]) && Number(bracket[2]) > 0 && Number(bracket[2]) <= 65535;
  const colon = value.lastIndexOf(":");
  if (colon < 0) return false;
  const [host, port] = [value.slice(0, colon), value.slice(colon + 1)];
  return host.length > 0 && host.length <= 253 && host.split(".").every((part) => label(part, /^[A-Za-z0-9-]+$/)) && /^\d+$/.test(port) && Number(port) > 0 && Number(port) <= 65535;
}

function isDomain(value: string): boolean {
  const bare = value.startsWith("~") ? value.slice(1) : value;
  if (bare === ".") return true;
  const name = bare.endsWith(".") ? bare.slice(0, -1) : bare;
  return name.length <= 253 && name.split(".").every((part) => label(part, /^[A-Za-z0-9_-]+$/));
}

// 32 bytes in base64: 43 characters and one `=`, the last character with its two spare bits at zero (what Rust's decoder asks for).
const isKey = (value: string): boolean => /^[A-Za-z0-9+/]{42}[AEIMQUYcgkosw048]=$/.test(value);

/** `null` for a number the field may leave empty; anything else that is not a whole number is not good. */
const notWhole = (n: number | null): boolean => n !== null && !Number.isInteger(n);

/** Everything wrong with the WireGuard settings, in the order of the form, each with the path of its field. */
export function checkWireguard(p: WireGuardView): FieldIssue[] {
  const out: FieldIssue[] = [];
  const add = (field: string, code: string) => out.push({ field, code });
  if (p.addresses.length === 0) add("addresses", "required");
  else if (!p.addresses.every(isCidr)) add("addresses", "invalid_cidr");
  if (!p.dns.every(isIp)) add("dns", "invalid_ip");
  if (!p.search_domains.every(isDomain)) add("search_domains", "invalid_domain");
  if (notWhole(p.mtu)) add("mtu", "invalid_number");
  else if (p.mtu !== null && (p.mtu < 576 || p.mtu > 9000)) add("mtu", "out_of_range");
  if (p.peers.length === 0) add("peers", "required");
  for (const [i, peer] of p.peers.entries()) {
    if (!isKey(peer.public_key)) add(`peers.${i}.public_key`, "invalid_key");
    if (peer.endpoint !== null && !isEndpoint(peer.endpoint)) add(`peers.${i}.endpoint`, "invalid_endpoint");
    if (peer.allowed_ips.length === 0) add(`peers.${i}.allowed_ips`, "required");
    else if (!peer.allowed_ips.every(isCidr)) add(`peers.${i}.allowed_ips`, "invalid_cidr");
    const keepalive = peer.persistent_keepalive;
    if (notWhole(keepalive)) add(`peers.${i}.persistent_keepalive`, "invalid_number");
    else if (keepalive !== null && (keepalive < 0 || keepalive > 65535)) add(`peers.${i}.persistent_keepalive`, "out_of_range");
  }
  return out;
}
