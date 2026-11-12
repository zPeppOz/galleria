use base64::{Engine, engine::general_purpose::STANDARD};
use galleria_backend::{WG_FWMARK, WG_INTERFACE, net::*};
use galleria_ipc::{ErrorCode, RpcError, WireGuardParams};
use ipnet::IpNet;
use std::net::IpAddr;
use zeroize::Zeroizing;
#[derive(Clone)]
pub struct ValidPeer {
    pub public_key: [u8; 32],
    pub preshared_key: Option<Zeroizing<[u8; 32]>>,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<IpNet>,
    pub persistent_keepalive: Option<u16>,
}
#[derive(Clone)]
pub struct ValidWireGuard {
    pub private_key: Zeroizing<[u8; 32]>,
    pub addresses: Vec<IpNet>,
    pub dns: Vec<IpAddr>,
    pub search_domains: Vec<String>,
    pub mtu: Option<u32>,
    pub peers: Vec<ValidPeer>,
}
fn invalid(field: &str) -> RpcError {
    let mut e = ErrorCode::InvalidConfig.to_rpc("Invalid WireGuard configuration");
    e.data = Some(serde_json::json!({"code":"invalid_config","field":field}));
    e
}
fn key(value: &str, field: &str) -> Result<Zeroizing<[u8; 32]>, RpcError> {
    let decoded = Zeroizing::new(STANDARD.decode(value).map_err(|_| invalid(field))?);
    let bytes = decoded.as_slice().try_into().map_err(|_| invalid(field))?;
    Ok(Zeroizing::new(bytes))
}
fn networks(values: &[String], field: &str) -> Result<Vec<IpNet>, RpcError> {
    values
        .iter()
        .map(|s| s.parse().map_err(|_| invalid(field)))
        .collect()
}
fn endpoint(value: &str) -> bool {
    if let Ok(addr) = value.parse::<std::net::SocketAddr>() {
        return addr.port() != 0;
    }
    let Some((host, port)) = value.rsplit_once(':') else {
        return false;
    };
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && port.parse::<u16>().is_ok_and(|n| n > 0)
}
fn domain(value: &str) -> bool {
    let v = value.strip_prefix('~').unwrap_or(value);
    if v == "." {
        return true;
    }
    let v = v.strip_suffix('.').unwrap_or(v);
    v.len() <= 253
        && v.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
}
pub fn validate(p: &WireGuardParams) -> Result<ValidWireGuard, RpcError> {
    let private_key = key(p.private_key.expose(), "private_key")?;
    let addresses = networks(&p.addresses, "addresses")?;
    if p.peers.is_empty() {
        return Err(invalid("peers"));
    }
    if p.mtu.is_some_and(|m| !(576..=9000).contains(&m)) {
        return Err(invalid("mtu"));
    }
    let dns = p
        .dns
        .iter()
        .map(|s| s.parse::<IpAddr>().map_err(|_| invalid("dns")))
        .collect::<Result<_, _>>()?;
    if !p.search_domains.iter().all(|s| domain(s)) {
        return Err(invalid("search_domains"));
    }
    let mut peers = Vec::new();
    for (i, p) in p.peers.iter().enumerate() {
        let field = |name: &str| format!("peers.{i}.{name}");
        let public_key = *key(&p.public_key, &field("public_key"))?;
        let preshared_key = p
            .preshared_key
            .as_ref()
            .map(|s| key(s.expose(), &field("preshared_key")))
            .transpose()?;
        if p.endpoint.as_ref().is_some_and(|s| !endpoint(s)) {
            return Err(invalid(&field("endpoint")));
        }
        let allowed_ips = networks(&p.allowed_ips, &field("allowed_ips"))?;
        let persistent_keepalive = p
            .persistent_keepalive
            .map(|n| u16::try_from(n).map_err(|_| invalid(&field("persistent_keepalive"))))
            .transpose()?;
        peers.push(ValidPeer {
            public_key,
            preshared_key,
            endpoint: p.endpoint.clone(),
            allowed_ips,
            persistent_keepalive,
        });
    }
    Ok(ValidWireGuard {
        private_key,
        addresses,
        dns,
        search_domains: p.search_domains.clone(),
        mtu: p.mtu,
        peers,
    })
}
pub fn net_config(cfg: &ValidWireGuard, ipv6_block_enabled: bool) -> NetConfig {
    let mut routes = Vec::new();
    for route in cfg.peers.iter().flat_map(|p| &p.allowed_ips) {
        if !routes.contains(route) {
            routes.push(*route);
        }
    }
    let block = ipv6_block_enabled && is_full_tunnel_v4(&routes) && !carries_ipv6_default(&routes);
    NetConfig {
        interface: WG_INTERFACE.into(),
        dns_servers: cfg.dns.clone(),
        search_domains: cfg.search_domains.clone(),
        routes,
        install_routes: Some(WG_FWMARK),
        ipv6_block: if block {
            Ipv6Block::Unreachable
        } else {
            Ipv6Block::None
        },
    }
}
