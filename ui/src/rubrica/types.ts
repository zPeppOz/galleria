import type { Protocol } from "../daemon/types";

// Hand-written mirror of the views of `crates/galleria-gui/src/rubrica.rs` (snake_case, as serialised). None of them
// has a secret: `secret_refs` holds the names of the secrets of a profile, never their values.

export interface ProfileSummary {
  id: string;
  name: string;
  protocol: Protocol;
  group: string | null;
  favorite: boolean;
  /** `host:port` of the first remote or of the endpoint of the first peer. */
  server: string | null;
  /** RFC 3339; nobody writes it before B5. */
  last_used: string | null;
  modified: string;
}

export interface Remote { host: string; port: number | null; transport: "udp" | "tcp" | null }
export interface AuthFlags { certificate: boolean; user_pass: boolean; otp: boolean }
export interface OpenVpnView { config: string; remotes: Remote[]; auth: AuthFlags }
export interface WireGuardPeer { public_key: string; endpoint: string | null; allowed_ips: string[]; persistent_keepalive: number | null }
export interface WireGuardView { addresses: string[]; dns: string[]; search_domains: string[]; mtu: number | null; peers: WireGuardPeer[] }
export interface ProfileSettings { ipv6_block: boolean; auto_reconnect: boolean; ignore_pushed_routes: boolean }

export interface ProfileDetail extends ProfileSummary {
  notes: string;
  tags: string[];
  created: string;
  settings: ProfileSettings;
  secret_refs: string[];
  openvpn: OpenVpnView | null;
  wireguard: WireGuardView | null;
}

/** A file of the profiles folder that could not be read, shown by its name. */
export interface SkippedProfile { file_name: string; code: "gui_profile_newer" | "gui_profile_invalid" }

export interface RubricaView { profiles: ProfileSummary[]; groups: string[]; skipped: SkippedProfile[] }

/** The error every command returns (spec §4): a stable code, the English message, and sometimes data for the text. */
export interface CmdError { code: string; message: string; data?: { path?: string; mode?: string; fix?: string } }
