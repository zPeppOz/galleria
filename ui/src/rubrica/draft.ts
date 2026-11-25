import type { FieldIssue, ProfileDetail, ProfilePatch, WireGuardView } from "./types";
import { checkLabel, checkWireguard, NAME_MAX } from "./validate";

/**
 * The profile as the form edits it: every field is text, as typed. Lists are one line with commas, numbers are the
 * digits typed (empty is «not set»), «no group» is the empty string. The form keeps a draft next to the saved profile;
 * `build` turns the difference into a patch, or into the list of fields that are wrong. Pure: no React, no Tauri.
 */
export interface PeerDraft { public_key: string; endpoint: string; allowed_ips: string; persistent_keepalive: string }
export interface WireGuardDraft { addresses: string; dns: string; search_domains: string; mtu: string; peers: PeerDraft[] }
export interface Draft { name: string; group: string; notes: string; wireguard: WireGuardDraft | null; /** The text of an OpenVPN profile as the editor has it; null for a WireGuard one. */ config: string | null }

const join = (items: string[]) => items.join(", ");
const split = (text: string) => text.split(/[\s,]+/).filter((item) => item !== "");
const digits = (n: number | null) => (n === null ? "" : String(n));
/** Empty is «not set»; text that is not a number is NaN, which the check calls `invalid_number`. */
const number = (text: string): number | null => (text.trim() === "" ? null : /^\d+$/.test(text.trim()) ? Number(text.trim()) : Number.NaN);

export function toDraft(detail: ProfileDetail): Draft {
  const wg = detail.wireguard;
  return {
    name: detail.name,
    group: detail.group ?? "",
    notes: detail.notes,
    config: detail.openvpn?.config ?? null,
    wireguard: wg && {
      addresses: join(wg.addresses),
      dns: join(wg.dns),
      search_domains: join(wg.search_domains),
      mtu: digits(wg.mtu),
      peers: wg.peers.map((p) => ({ public_key: p.public_key, endpoint: p.endpoint ?? "", allowed_ips: join(p.allowed_ips), persistent_keepalive: digits(p.persistent_keepalive) })),
    },
  };
}

export const isDirty = (draft: Draft, base: ProfileDetail): boolean => JSON.stringify(draft) !== JSON.stringify(toDraft(base));

const toView = (draft: WireGuardDraft): WireGuardView => ({
  addresses: split(draft.addresses),
  dns: split(draft.dns),
  search_domains: split(draft.search_domains),
  mtu: number(draft.mtu),
  peers: draft.peers.map((p) => ({
    public_key: p.public_key.trim(),
    endpoint: p.endpoint.trim() === "" ? null : p.endpoint.trim(),
    allowed_ips: split(p.allowed_ips),
    persistent_keepalive: number(p.persistent_keepalive),
  })),
});

/** The patch with only what changed, and what is wrong with it. A field that is wrong is not in the patch. */
export function build(draft: Draft, base: ProfileDetail): { patch: ProfilePatch; issues: FieldIssue[] } {
  const patch: ProfilePatch = {};
  const issues: FieldIssue[] = [];
  const before = toDraft(base);
  const name = draft.name.trim();
  if (draft.name !== before.name) {
    const code = checkLabel(name, NAME_MAX);
    if (code) issues.push({ field: "name", code });
    else if (name !== base.name) patch.name = name;
  }
  if (draft.group !== before.group) patch.group = draft.group === "" ? null : draft.group;
  if (draft.notes !== before.notes) patch.notes = draft.notes;
  // The text goes as it is typed: no trimming, no tidying. What is checked is checked by the editor and again by Rust.
  if (draft.config !== null && draft.config !== before.config) patch.openvpn_config = draft.config;
  if (draft.wireguard && JSON.stringify(draft.wireguard) !== JSON.stringify(before.wireguard)) {
    const view = toView(draft.wireguard);
    const found = checkWireguard(view);
    if (found.length === 0) patch.wireguard = view;
    issues.push(...found);
  }
  return { patch, issues };
}

/** The fields in the order the tabs show them (General, Authentication, Network): the first wrong one gets the focus. */
export function fieldOrder(peers: number): string[] {
  const each = (name: string) => Array.from({ length: peers }, (_, i) => `peers.${i}.${name}`);
  const network = Array.from({ length: peers }, (_, i) => [`peers.${i}.allowed_ips`, `peers.${i}.persistent_keepalive`]).flat();
  return ["name", "group", "notes", ...each("endpoint"), ...each("public_key"), "addresses", "dns", "search_domains", "mtu", ...network, "openvpn_config"];
}

export type DetailTab = "general" | "auth" | "network" | "config";

/** The tab a field is in (D26): the endpoint of a peer is in General, its key in Authentication, the text in Configuration, the rest in Network. */
export function fieldTab(field: string): DetailTab {
  if (field === "name" || field === "group" || field === "notes" || field.endsWith(".endpoint")) return "general";
  if (field.endsWith(".public_key")) return "auth";
  if (field === "openvpn_config") return "config";
  return "network";
}
