#![forbid(unsafe_code)]
mod import_ovpn;
mod import_wg;
mod model;
mod secrets;
mod settings;
mod store;
use galleria_ipc::{
    BackendParams, ConnectOptions, ConnectParams, OpenVpnParams, ProfileRef, Protocol,
    ReconnectOptions, Secret, WireGuardParams,
};
pub use import_ovpn::*;
pub use import_wg::*;
pub use model::*;
pub use secrets::*;
pub use settings::*;
pub use store::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningKind {
    Dangerous,
    Unsupported,
    NoTlsClient,
}
#[derive(Debug)]
pub struct ImportWarning {
    pub line: usize,
    pub directive: String,
    pub kind: WarningKind,
}
pub struct Imported {
    pub profile: Profile,
    pub secrets: Vec<(String, Secret)>,
    pub warnings: Vec<ImportWarning>,
}
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Invalid VPN profile at line {0}")]
    Syntax(usize),
    #[error("Cannot read referenced file: {0}")]
    MissingFile(String),
    #[error("Referenced file is outside the profile folder: {0}")]
    OutsideFolder(String),
    #[error("Invalid VPN profile")]
    Invalid,
}
pub fn to_connect_params(
    profile: &Profile,
    secrets: &dyn SecretStore,
    replace: bool,
) -> Result<ConnectParams, StoreError> {
    let missing = || StoreError::Invalid("Profile is missing required key material".into());
    let backend = match profile.protocol {
        Protocol::Openvpn => {
            let p = profile.openvpn.as_ref().ok_or_else(missing)?;
            let mut blocks = std::collections::BTreeMap::new();
            for tag in galleria_ovpn::SECRET_BLOCKS {
                let key = format!("openvpn.block.{tag}");
                if profile.secret_refs.contains(&key) {
                    blocks.insert(
                        (*tag).into(),
                        secrets.get(&profile.id, &key)?.ok_or_else(missing)?,
                    );
                }
            }
            BackendParams::Openvpn(OpenVpnParams {
                config: p.config.clone(),
                secret_blocks: blocks,
                username: None,
                password: None,
                otp: None,
                key_passphrase: None,
            })
        }
        Protocol::Wireguard => {
            let p = profile.wireguard.as_ref().ok_or_else(missing)?;
            let mut peers = vec![];
            for (i, peer) in p.peers.iter().enumerate() {
                let key = format!("wireguard.peer.{i}.preshared_key");
                let psk = if profile.secret_refs.contains(&key) {
                    Some(secrets.get(&profile.id, &key)?.ok_or_else(missing)?)
                } else {
                    None
                };
                peers.push(galleria_ipc::WireGuardPeer {
                    public_key: peer.public_key.clone(),
                    preshared_key: psk,
                    endpoint: peer.endpoint.clone(),
                    allowed_ips: peer.allowed_ips.clone(),
                    persistent_keepalive: peer.persistent_keepalive,
                });
            }
            BackendParams::Wireguard(WireGuardParams {
                private_key: secrets
                    .get(&profile.id, "wireguard.private_key")?
                    .ok_or_else(missing)?,
                addresses: p.addresses.clone(),
                dns: p.dns.clone(),
                search_domains: p.search_domains.clone(),
                mtu: p.mtu,
                peers,
            })
        }
    };
    Ok(ConnectParams {
        profile: ProfileRef {
            id: profile.id.to_string(),
            name: profile.name.clone(),
        },
        replace,
        backend,
        options: ConnectOptions {
            ipv6_block: profile.settings.ipv6_block,
            ignore_pushed_routes: profile.settings.ignore_pushed_routes,
            reconnect: ReconnectOptions {
                enabled: profile.settings.auto_reconnect,
            },
        },
    })
}
