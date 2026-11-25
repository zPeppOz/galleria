import type { Protocol } from "./types";

/** The names of the protocols are brand names: the same in every language, so they are not in the catalogues. */
export const PROTOCOL_NAME: Record<Protocol, string> = { wireguard: "WireGuard", openvpn: "OpenVPN" };

/** The short tag shown on a row and in the header; the full name goes to assistive technology next to it. */
export const PROTOCOL_TAG: Record<Protocol, string> = { wireguard: "WG", openvpn: "OVPN" };
