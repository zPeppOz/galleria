use crate::config::{Redirect, redirect};
use ipnet::{IpNet, Ipv4Net};
use std::net::{IpAddr, Ipv4Addr};
#[derive(Debug, Clone, Default)]
pub struct Pushed {
    pub redirect: Redirect,
    pub continuation: bool,
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
            "push-continuation" => p.continuation = args.first().is_some_and(|a| a == "2"),
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

/// One accumulator per log transport: stdout and management can deliver duplicates
/// at different times, so a late first fragment must not replace a complete reply.
#[derive(Default)]
pub(crate) struct PushAccumulator(Option<Pushed>);
impl PushAccumulator {
    pub fn clear(&mut self) {
        self.0 = None;
    }
    pub fn feed(&mut self, text: &str, ignore_routes: bool) -> Option<Pushed> {
        let mut part = parse_push_reply(text)?;
        let mut full = self.0.take().unwrap_or_default();
        full.redirect.ipv4 |= part.redirect.ipv4;
        full.redirect.ipv6 |= part.redirect.ipv6;
        full.routes.append(&mut part.routes);
        full.dns_servers.append(&mut part.dns_servers);
        full.ifconfig_ipv6 |= part.ifconfig_ipv6;
        if part.continuation {
            self.0 = Some(full);
            None
        } else {
            if ignore_routes {
                full.redirect = Redirect::default();
                full.routes.clear();
            }
            Some(full)
        }
    }
}
