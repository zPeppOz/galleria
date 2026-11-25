import type { AppInfo, DaemonView, Status } from "../daemon/types";
import type { ProfileDetail, ProfilePatch, ProfileSummary, RubricaView } from "../rubrica/types";

export function makeStatus(over: Partial<Status> = {}): Status {
  return {
    state: "disconnected",
    session: null,
    profile: null,
    protocol: null,
    since: null,
    attempt: 0,
    detail: null,
    local_addresses: [],
    remote: null,
    net: null,
    stats: null,
    input_request: null,
    error: null,
    ...over,
  };
}

export function makeView(over: Partial<DaemonView> = {}): DaemonView {
  return {
    link: "connected",
    daemon_version: "0.1.0",
    incompatible: null,
    backends: [],
    status: makeStatus(),
    ...over,
  };
}

export function makeInfo(over: Partial<AppInfo> = {}): AppInfo {
  return {
    version: "0.1.0",
    socket_path: "/run/galleria/galleriad.sock",
    user: "mrossi",
    group: "galleria",
    ...over,
  };
}

export function makeSummary(over: Partial<ProfileSummary> = {}): ProfileSummary {
  return {
    id: "p1",
    name: "Prova",
    protocol: "wireguard",
    group: null,
    favorite: false,
    server: "prova.example:51820",
    last_used: null,
    modified: "2026-11-25T10:00:00Z",
    ...over,
  };
}

/** The five profiles of the mockup, as the backend would list them, in the three groups of the seeded rubrica. */
export const SAMPLE_PROFILES: ProfileSummary[] = [
  makeSummary({ id: "office", name: "Ufficio Napoli", protocol: "openvpn", group: "Lavoro", favorite: true, server: "vpn.ufficio.example:1194" }),
  makeSummary({ id: "datacenter", name: "Datacenter Milano", protocol: "openvpn", group: "Lavoro", server: "gw.dc-mi.example:443" }),
  makeSummary({ id: "homelab", name: "Homelab", protocol: "wireguard", group: "Personale", favorite: true, server: "casa.example:51820" }),
  makeSummary({ id: "vps", name: "VPS Hetzner", protocol: "wireguard", group: "Personale", server: "203.0.113.24:51820" }),
  makeSummary({ id: "rossi", name: "Cliente Rossi", protocol: "openvpn", group: "Clienti", server: "vpn.rossi.example:1194" }),
];

export function makeRubricaView(over: Partial<RubricaView> = {}): RubricaView {
  return { profiles: SAMPLE_PROFILES, groups: ["Lavoro", "Personale", "Clienti"], skipped: [], ...over };
}

/** The detail of a summary: an OpenVPN one gets a configuration with its remote, a WireGuard one a single peer. */
export function makeDetail(summary: ProfileSummary): ProfileDetail {
  const [host, port] = (summary.server ?? "").split(":");
  const base = {
    ...summary,
    notes: "",
    tags: [],
    created: "2026-11-20T09:00:00Z",
    settings: { ipv6_block: true, auto_reconnect: true, ignore_pushed_routes: false },
    secret_refs: [],
  };
  if (summary.protocol === "openvpn") {
    const udp = summary.id !== "datacenter";
    return {
      ...base,
      openvpn: {
        config: `client\nproto ${udp ? "udp" : "tcp"}\nremote ${host} ${port}\n`,
        remotes: host ? [{ host, port: Number(port), transport: udp ? "udp" : "tcp" }] : [],
        auth: { certificate: true, user_pass: false, otp: false },
      },
      wireguard: null,
    };
  }
  return {
    ...base,
    openvpn: null,
    wireguard: {
      addresses: ["10.8.0.2/32"],
      dns: [],
      search_domains: [],
      mtu: null,
      peers: [{ public_key: "PUBLIC", endpoint: summary.server, allowed_ips: ["0.0.0.0/0"], persistent_keepalive: 25 }],
    },
  };
}

/** What the backend answers to an update: the detail with the patch on it and a new time of modification. */
export function applyPatch(detail: ProfileDetail, patch: ProfilePatch): ProfileDetail {
  const { group, wireguard, ...rest } = patch;
  return {
    ...detail,
    ...rest,
    group: group === undefined ? detail.group : group,
    wireguard: wireguard ?? detail.wireguard,
    modified: `${detail.modified}+`,
  };
}
