use crate::{
    config::Redirect,
    push::{Pushed, route_v4},
};
use galleria_backend::{OVPN_INTERFACE, net::*};
use ipnet::IpNet;
use std::{collections::BTreeMap, net::IpAddr};
pub const HOOK_ENV_PREFIXES: &[&str] = &[
    "dev",
    "script_type",
    "foreign_option_",
    "ifconfig_",
    "route_",
    "redirect_gateway",
    "dns_",
    "tun_mtu",
];
#[derive(Debug, Clone)]
pub struct HookEnv {
    pub script_type: String,
    pub dev: String,
    pub dns_servers: Vec<IpAddr>,
    pub search_domains: Vec<String>,
    pub routes: Vec<IpNet>,
    pub has_ipv6_address: bool,
}
pub fn parse_hook_env(env: &BTreeMap<String, String>) -> Result<HookEnv, String> {
    let script_type = env
        .get("script_type")
        .filter(|s| s.as_str() == "up" || s.as_str() == "down")
        .ok_or("Invalid hook type")?
        .clone();
    let dev = env
        .get("dev")
        .filter(|s| s.as_str() == OVPN_INTERFACE)
        .ok_or("Invalid hook interface")?
        .clone();
    let mut h = HookEnv {
        script_type,
        dev,
        dns_servers: vec![],
        search_domains: vec![],
        routes: vec![],
        has_ipv6_address: env
            .get("ifconfig_ipv6_local")
            .is_some_and(|s| s.parse::<IpAddr>().is_ok()),
    };
    for (key, value) in env {
        if key.starts_with("foreign_option_") {
            let words = galleria_ovpn::words(value).map_err(|_| "Invalid foreign option")?;
            if words.first().is_some_and(|s| s == "dhcp-option") && words.len() >= 3 {
                match words[1].as_str() {
                    "DNS" | "DNS6" => h
                        .dns_servers
                        .push(words[2].parse().map_err(|_| "Invalid DNS address")?),
                    "DOMAIN" | "DOMAIN-SEARCH" => {
                        for d in &words[2..] {
                            if d.is_empty()
                                || d.starts_with('-')
                                || !d
                                    .bytes()
                                    .all(|c| c.is_ascii_alphanumeric() || b".-_~".contains(&c))
                            {
                                return Err("Invalid search domain".into());
                            }
                            h.search_domains.push(d.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(n) = key.strip_prefix("route_network_")
            && let Some(mask) = env.get(&format!("route_netmask_{n}"))
        {
            h.routes
                .push(route_v4(value, mask).ok_or("Invalid IPv4 route")?);
        }
        if key.starts_with("route_ipv6_network_") {
            h.routes
                .push(value.parse().map_err(|_| "Invalid IPv6 route")?);
        }
    }
    Ok(h)
}
pub struct ProfileFacts {
    pub redirect: Redirect,
    pub ipv6_routes: Vec<IpNet>,
}
pub fn net_config(
    env: &HookEnv,
    pushed: Option<&Pushed>,
    profile: &ProfileFacts,
    ipv6_block_enabled: bool,
) -> NetConfig {
    let mut routes = env.routes.clone();
    routes.extend(profile.ipv6_routes.iter().copied());
    let mut redirect = profile.redirect;
    if let Some(p) = pushed {
        routes.extend(p.routes.iter().copied());
        redirect.ipv4 |= p.redirect.ipv4;
        redirect.ipv6 |= p.redirect.ipv6;
    }
    if redirect.ipv4
        && let Ok(n) = "0.0.0.0/0".parse()
    {
        routes.push(n);
    }
    if redirect.ipv6 {
        for s in ["2000::/4", "3000::/4"] {
            if let Ok(n) = s.parse() {
                routes.push(n);
            }
        }
    }
    routes.sort();
    routes.dedup();
    let ipv6_block = if ipv6_block_enabled
        && redirect.ipv4
        && !redirect.ipv6
        && !carries_ipv6_default(&routes)
    {
        Ipv6Block::Unreachable
    } else {
        Ipv6Block::None
    };
    NetConfig {
        interface: OVPN_INTERFACE.into(),
        dns_servers: env.dns_servers.clone(),
        search_domains: env.search_domains.clone(),
        routes,
        install_routes: None,
        ipv6_block,
    }
}
