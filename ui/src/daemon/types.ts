// Hand-written mirror of the Rust types the commands return (snake_case field names, as serialized).
export type LinkState = "connecting" | "connected" | "daemon_not_running" | "permission_denied" | "incompatible_protocol";
export type VpnState = "disconnected" | "connecting" | "needs_input" | "connected" | "reconnecting" | "disconnecting" | "error";
export const VPN_STATES: VpnState[] = ["disconnected", "connecting", "needs_input", "connected", "reconnecting", "disconnecting", "error"];
export type Protocol = "wireguard" | "openvpn";

export interface NetInfo { interface: string; dns_servers: string[]; search_domains: string[]; routes: string[]; full_tunnel: boolean; ipv6_block: "none" | "unreachable" }
export interface SessionError { code: string; message: string }
export interface Status {
  state: VpnState; session: number | null; profile: { id: string; name: string } | null; protocol: Protocol | null;
  since: string | null; attempt: number; detail: string | null; local_addresses: string[]; remote: string | null;
  net: NetInfo | null; stats: { rx_bytes: number; tx_bytes: number; last_handshake: string | null } | null;
  input_request: unknown; error: SessionError | null;
}
export interface BackendInfo {
  protocol: Protocol; available: boolean; version: string | null; unavailable_reason: string | null;
  capabilities: { handshake: boolean; interactive_auth: boolean; server_pushed_config: boolean };
}
export interface Incompatible { daemon_min: number; daemon_max: number; daemon_version: string }
export interface DaemonView { link: LinkState; daemon_version: string | null; incompatible: Incompatible | null; backends: BackendInfo[]; status: Status | null }
export interface AppInfo { version: string; socket_path: string; user: string; group: string }

/** The codes a session can end with (`Status.error.code`); the catalogues have a text for each (`error.<code>`). */
export const SESSION_ERROR_CODES = ["auth_failed", "input_timeout", "server_unreachable", "tunnel_setup_failed", "net_setup_failed", "dns_unavailable", "unsupported_challenge", "backend_exited"] as const;
