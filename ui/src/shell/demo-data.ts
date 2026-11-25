import type { Protocol } from "../daemon/types";

/**
 * The invented profiles of the shell of B2 (the five of the mockup, on `example` domains). They are not real, they carry no
 * secret, not even a made-up one, and nothing reads them from the daemon: B3 replaces them with the profiles that exist.
 */
export type DemoGroup = "work" | "personal" | "clients";

export interface DemoProfile {
  id: string;
  name: string;
  protocol: Protocol;
  group: DemoGroup;
  favourite?: boolean;
  server: string;
  /** OpenVPN only. */
  transport?: "UDP" | "TCP";
}

export const DEMO_GROUPS: DemoGroup[] = ["work", "personal", "clients"];

export const DEMO_PROFILES: DemoProfile[] = [
  { id: "office", name: "Ufficio Napoli", protocol: "openvpn", group: "work", favourite: true, server: "vpn.ufficio.example:1194", transport: "UDP" },
  { id: "datacenter", name: "Datacenter Milano", protocol: "openvpn", group: "work", server: "gw.dc-mi.example:443", transport: "TCP" },
  { id: "homelab", name: "Homelab", protocol: "wireguard", group: "personal", favourite: true, server: "casa.example:51820" },
  { id: "vps", name: "VPS Hetzner", protocol: "wireguard", group: "personal", server: "203.0.113.24:51820" },
  { id: "rossi", name: "Cliente Rossi", protocol: "openvpn", group: "clients", server: "vpn.rossi.example:1194", transport: "UDP" },
];
