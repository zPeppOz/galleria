use crate::{Secret, SessionError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Wireguard,
    Openvpn,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Disconnected,
    Connecting,
    NeedsInput,
    Connected,
    Reconnecting,
    Disconnecting,
    Error,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub state: State,
    pub session: Option<u64>,
    pub profile: Option<ProfileRef>,
    pub protocol: Option<Protocol>,
    pub since: Option<String>,
    pub attempt: u32,
    pub detail: Option<String>,
    pub local_addresses: Vec<std::net::IpAddr>,
    pub remote: Option<std::net::SocketAddr>,
    pub net: Option<NetInfo>,
    pub stats: Option<Stats>,
    pub input_request: Option<InputRequest>,
    pub error: Option<SessionError>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            state: State::Disconnected,
            session: None,
            profile: None,
            protocol: None,
            since: None,
            attempt: 0,
            detail: None,
            local_addresses: vec![],
            remote: None,
            net: None,
            stats: None,
            input_request: None,
            error: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetInfo {
    pub interface: String,
    pub dns_servers: Vec<std::net::IpAddr>,
    pub search_domains: Vec<String>,
    pub routes: Vec<String>,
    pub full_tunnel: bool,
    pub ipv6_block: Ipv6BlockInfo,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ipv6BlockInfo {
    None,
    Unreachable,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Stats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub last_handshake: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputRequest {
    pub request: u64,
    pub kind: InputKind,
    pub prompt: String,
    pub echo: bool,
    pub expires_in_secs: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Credentials,
    Otp,
    KeyPassphrase,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputValues {
    pub username: Option<String>,
    pub password: Option<Secret>,
    pub otp: Option<Secret>,
    pub key_passphrase: Option<Secret>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub seq: u64,
    pub ts: String,
    pub level: LogLevel,
    pub source: LogSource,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "protocol", rename_all = "snake_case")]
pub enum BackendParams {
    Wireguard(WireGuardParams),
    Openvpn(OpenVpnParams),
}
impl BackendParams {
    pub fn protocol(&self) -> Protocol {
        match self {
            Self::Wireguard(_) => Protocol::Wireguard,
            Self::Openvpn(_) => Protocol::Openvpn,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireGuardParams {
    pub private_key: Secret,
    pub addresses: Vec<String>,
    pub dns: Vec<String>,
    pub search_domains: Vec<String>,
    pub mtu: Option<u32>,
    pub peers: Vec<WireGuardPeer>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireGuardPeer {
    pub public_key: String,
    pub preshared_key: Option<Secret>,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenVpnParams {
    pub config: String,
    pub secret_blocks: BTreeMap<String, Secret>,
    pub username: Option<String>,
    pub password: Option<Secret>,
    pub otp: Option<Secret>,
    pub key_passphrase: Option<Secret>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectOptions {
    pub ipv6_block: bool,
    pub ignore_pushed_routes: bool,
    pub reconnect: ReconnectOptions,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconnectOptions {
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRef {
    pub id: String,
    pub name: String,
}

// ProfileRef is also a response type, so only the request field uses strict decoding.
fn request_profile<'de, D: serde::Deserializer<'de>>(d: D) -> Result<ProfileRef, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        id: String,
        name: String,
    }
    let p = Strict::deserialize(d)?;
    Ok(ProfileRef {
        id: p.id,
        name: p.name,
    })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelloParams {
    pub protocol: u32,
    pub client: ClientInfo,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectParams {
    #[serde(deserialize_with = "request_profile")]
    pub profile: ProfileRef,
    pub replace: bool,
    pub backend: BackendParams,
    pub options: ConnectOptions,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvideInputParams {
    pub session: u64,
    pub request: u64,
    pub values: InputValues,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscribeParams {
    pub stats: bool,
    pub logs: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogsParams {
    pub after: Option<u64>,
    pub limit: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResult {
    pub protocol: u32,
    pub daemon_version: String,
    pub backends: Vec<BackendInfo>,
    pub status: Status,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    pub protocol: Protocol,
    pub available: bool,
    pub version: Option<String>,
    pub unavailable_reason: Option<String>,
    pub capabilities: Capabilities,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub handshake: bool,
    pub interactive_auth: bool,
    pub server_pushed_config: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectResult {
    pub session: u64,
    pub status: Status,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsResult {
    pub entries: Vec<LogEntry>,
    pub last: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    Daemon,
    Openvpn,
    Wireguard,
    Net,
}
#[derive(Debug, Clone, PartialEq)]
// Keep the owned state variant specified by the public IPC interface.
#[allow(clippy::large_enum_variant)]
pub enum Event {
    State(Status),
    Stats(Stats),
    Log(LogEntry),
}
