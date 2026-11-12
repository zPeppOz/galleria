use crate::config::{Redirect, redirect};
use ipnet::{IpNet, Ipv4Net};
use std::net::{IpAddr, Ipv4Addr};
#[derive(Debug, Clone, Default)]
pub struct Pushed {
    pub redirect: Redirect,
    pub routes: Vec<IpNet>,
    pub ifconfig_ipv6: bool,
    pub dns_servers: Vec<IpAddr>,
}
pub(crate) fn route_v4(address: &str, mask: &str) -> Option<IpNet> {
    Some(
        Ipv4Net::with_netmask(address.parse::<Ipv4Addr>().ok()?, mask.parse().ok()?)
            .ok()?
            .trunc()
            .into(),
    )
}
pub fn parse_push_reply(text: &str) -> Option<Pushed> {
    let (_, s) = text.split_once("PUSH_REPLY")?;
    if !s.is_empty() && !s.starts_with(',') {
        return None;
    }
    let mut p = Pushed::default();
    for option in s.trim_end_matches('\'').split(',').skip(1) {
        let Ok(words) = galleria_ovpn::words(option) else {
            continue;
        };
        let Some((name, args)) = words.split_first() else {
            continue;
        };
        match name.as_str() {
            "redirect-gateway" => {
                let r = redirect(args);
                p.redirect.ipv4 |= r.ipv4;
                p.redirect.ipv6 |= r.ipv6;
            }
            "route" => {
                if let Some(r) = args
                    .first()
                    .zip(args.get(1))
                    .and_then(|(a, m)| route_v4(a, m))
                {
                    p.routes.push(r);
                }
            }
            "route-ipv6" => {
                if let Some(r) = args.first().and_then(|a| a.parse().ok()) {
                    p.routes.push(r);
                }
            }
            "ifconfig-ipv6" => p.ifconfig_ipv6 = true,
            "dhcp-option" if args.len() >= 2 && ["DNS", "DNS6"].contains(&args[0].as_str()) => {
                if let Ok(ip) = args[1].parse() {
                    p.dns_servers.push(ip);
                }
            }
            _ => {}
        }
    }
    Some(p)
}
