use crate::StoreError;
use galleria_ipc::Protocol;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub protocol: Protocol,
    pub group: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub notes: String,
    pub created: String,
    pub modified: String,
    pub secret_refs: Vec<String>,
    pub settings: Settings,
    pub openvpn: Option<OpenVpnProfile>,
    pub wireguard: Option<WireGuardProfile>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub ipv6_block: bool,
    pub auto_reconnect: bool,
    pub ignore_pushed_routes: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            ipv6_block: true,
            auto_reconnect: true,
            ignore_pushed_routes: false,
        }
    }
}
#[derive(Debug, Clone)]
pub enum ProtocolConfig {
    Openvpn(OpenVpnProfile),
    Wireguard(WireGuardProfile),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenVpnProfile {
    pub config: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireGuardProfile {
    pub addresses: Vec<String>,
    pub dns: Vec<String>,
    pub search_domains: Vec<String>,
    pub mtu: Option<u32>,
    pub peers: Vec<WireGuardPeer>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireGuardPeer {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u32>,
}
impl Profile {
    pub fn new(name: &str, config: ProtocolConfig) -> Self {
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let (protocol, openvpn, wireguard) = match config {
            ProtocolConfig::Openvpn(p) => (Protocol::Openvpn, Some(p), None),
            ProtocolConfig::Wireguard(p) => (Protocol::Wireguard, None, Some(p)),
        };
        Self {
            schema_version: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            name: name.into(),
            protocol,
            group: None,
            tags: vec![],
            favorite: false,
            notes: String::new(),
            created: timestamp.clone(),
            modified: timestamp,
            secret_refs: vec![],
            settings: Settings::default(),
            openvpn,
            wireguard,
        }
    }
    fn validate(&self) -> Result<(), StoreError> {
        if self.schema_version > SCHEMA_VERSION {
            return Err(StoreError::NewerSchema {
                found: self.schema_version,
            });
        }
        let shape = match self.protocol {
            Protocol::Openvpn => self.openvpn.is_some() && self.wireguard.is_none(),
            Protocol::Wireguard => self.wireguard.is_some() && self.openvpn.is_none(),
        };
        if self.schema_version != SCHEMA_VERSION || !shape {
            return Err(StoreError::Invalid(
                "Invalid profile schema or protocol".into(),
            ));
        }
        Ok(())
    }
    pub fn from_toml(text: &str) -> Result<Self, StoreError> {
        // Inspect the version before applying this version's strict field schema.
        let value: toml::Value =
            toml::from_str(text).map_err(|_| StoreError::Invalid("Invalid profile TOML".into()))?;
        if let Some(n) = value
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            && n > i64::from(SCHEMA_VERSION)
        {
            return Err(StoreError::NewerSchema {
                found: u32::try_from(n)
                    .map_err(|_| StoreError::Invalid("Invalid schema version".into()))?,
            });
        }
        let p: Self = value
            .try_into()
            .map_err(|_| StoreError::Invalid("Invalid or unknown profile field".into()))?;
        p.validate()?;
        Ok(p)
    }
    pub fn to_toml(&self) -> Result<String, StoreError> {
        self.validate()?;
        toml::to_string_pretty(self)
            .map_err(|_| StoreError::Invalid("Cannot encode profile".into()))
    }
}
