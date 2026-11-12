use crate::NetError;
use galleria_backend::{
    OVPN_INTERFACE, WG_FWMARK, WG_INTERFACE,
    net::{Ipv6Block, NetConfig, carries_ipv6_default, is_full_tunnel_v4},
};
use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Program {
    Ip,
    Resolvectl,
    Sysctl,
}
impl Program {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ip => "ip",
            Self::Resolvectl => "resolvectl",
            Self::Sysctl => "sysctl",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cmd {
    pub program: Program,
    pub args: Vec<String>,
}
impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.program.name(), self.args.join(" "))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub apply: Cmd,
    pub undo: Option<Cmd>,
}
fn cmd(program: Program, args: Vec<String>) -> Cmd {
    Cmd { program, args }
}
fn words(program: Program, args: &str) -> Cmd {
    cmd(
        program,
        args.split_whitespace().map(str::to_owned).collect(),
    )
}
fn step(apply: Cmd, undo: Option<Cmd>) -> Step {
    Step { apply, undo }
}
pub(crate) fn interface(s: &str) -> bool {
    matches!(s, WG_INTERFACE | OVPN_INTERFACE)
}
pub(crate) fn domain(s: &str) -> bool {
    let name = s.strip_prefix('~').unwrap_or(s);
    if name == "." {
        return true;
    }
    let name = name.strip_suffix('.').unwrap_or(name);
    name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        })
}
pub(crate) fn validate(cfg: &NetConfig) -> Result<(), NetError> {
    if !interface(&cfg.interface) {
        return Err(NetError::InvalidConfig(
            "interface must be a Galleria interface",
        ));
    }
    if cfg.install_routes.is_some_and(|mark| mark != WG_FWMARK) {
        return Err(NetError::InvalidConfig("unexpected routing table"));
    }
    if !cfg.search_domains.iter().all(|s| domain(s)) {
        return Err(NetError::InvalidConfig("invalid DNS domain"));
    }
    Ok(())
}
pub fn plan(cfg: &NetConfig) -> Vec<Step> {
    // The total, pure interface returns no commands for invalid input. apply reports the error.
    if validate(cfg).is_err() {
        return vec![];
    }
    let mut steps = Vec::new();
    if cfg.install_routes.is_some() {
        let full4 = is_full_tunnel_v4(&cfg.routes);
        let full6 = carries_ipv6_default(&cfg.routes);
        for route in &cfg.routes {
            let (family, full) = if route.addr().is_ipv4() {
                ("-4", full4)
            } else {
                ("-6", full6)
            };
            let mut args = vec![
                family.into(),
                "route".into(),
                "add".into(),
                route.to_string(),
                "dev".into(),
                cfg.interface.clone(),
            ];
            if full && full_tunnel_route(route, &cfg.routes) {
                args.extend(["table".into(), WG_FWMARK.to_string()]);
            }
            steps.push(step(cmd(Program::Ip, args), None));
        }
        for (family, full) in [("-4", full4), ("-6", full6)] {
            if full {
                for args in [
                    format!("{family} rule add not fwmark 51820 table 51820"),
                    format!("{family} rule add table main suppress_prefixlen 0"),
                ] {
                    steps.push(step(
                        words(Program::Ip, &args),
                        Some(words(Program::Ip, &args.replace(" add ", " del "))),
                    ));
                }
                if family == "-4" {
                    steps.push(step(
                        words(Program::Sysctl, "-q -w net.ipv4.conf.all.src_valid_mark=1"),
                        None,
                    ));
                }
            }
        }
    }
    if cfg.ipv6_block == Ipv6Block::Unreachable {
        for cidr in ["::/1", "8000::/1"] {
            steps.push(step(
                words(Program::Ip, &format!("-6 route replace unreachable {cidr}")),
                Some(words(
                    Program::Ip,
                    &format!("-6 route del unreachable {cidr}"),
                )),
            ));
        }
    }
    if !cfg.dns_servers.is_empty() {
        let mut args = vec!["dns".into(), cfg.interface.clone()];
        args.extend(cfg.dns_servers.iter().map(ToString::to_string));
        steps.push(step(
            cmd(Program::Resolvectl, args),
            Some(cmd(
                Program::Resolvectl,
                vec!["revert".into(), cfg.interface.clone()],
            )),
        ));
        let mut args = vec!["domain".into(), cfg.interface.clone()];
        args.extend(cfg.search_domains.clone());
        if is_full_tunnel_v4(&cfg.routes) || carries_ipv6_default(&cfg.routes) {
            args.push("~.".into());
        }
        steps.push(step(cmd(Program::Resolvectl, args), None));
        steps.push(step(
            cmd(
                Program::Resolvectl,
                vec!["default-route".into(), cfg.interface.clone(), "yes".into()],
            ),
            None,
        ));
    }
    steps
}
// Default routes (including split equivalents) use policy routing. More specific
// destinations remain in main so suppress_prefixlen cannot bypass the VPN route.
fn full_tunnel_route(route: &ipnet::IpNet, routes: &[ipnet::IpNet]) -> bool {
    if routes.iter().any(|other| {
        other.addr().is_ipv4() == route.addr().is_ipv4()
            && other.prefix_len() < route.prefix_len()
            && other.contains(&route.network())
    }) {
        return false;
    }
    match route {
        ipnet::IpNet::V4(_) => true,
        ipnet::IpNet::V6(net) => {
            let first = 0x2000_u128 << 112;
            let last = (0x4000_u128 << 112) - 1;
            net.prefix_len() <= 1
                || (u128::from(net.network()) <= last && u128::from(net.broadcast()) >= first)
        }
    }
}
pub fn fixed_cleanup() -> Vec<Cmd> {
    let mut cmds = vec![
        words(Program::Ip, "link del glr-wg0"),
        words(Program::Ip, "link del glr-ovpn0"),
    ];
    for family in ["-4", "-6"] {
        cmds.push(words(
            Program::Ip,
            &format!("{family} rule del not fwmark 51820 table 51820"),
        ));
        cmds.push(words(
            Program::Ip,
            &format!("{family} rule del table main suppress_prefixlen 0"),
        ));
    }
    cmds
}
impl Cmd {
    // A journal is data, never an executable command source. The production runner enforces
    // the same grammar even if called directly instead of through NetManager.
    pub(crate) fn validate(&self) -> Result<(), NetError> {
        let args = self.args.iter().map(String::as_str).collect::<Vec<_>>();
        let valid = match (self.program, args.as_slice()) {
            (Program::Ip, ["link", "del", iface]) => interface(iface),
            (
                Program::Ip,
                [
                    family,
                    "rule",
                    action,
                    "not",
                    "fwmark",
                    "51820",
                    "table",
                    "51820",
                ],
            )
            | (
                Program::Ip,
                [
                    family,
                    "rule",
                    action,
                    "table",
                    "main",
                    "suppress_prefixlen",
                    "0",
                ],
            ) => matches!(*family, "-4" | "-6") && matches!(*action, "add" | "del"),
            (Program::Ip, ["-6", "route", action, "unreachable", cidr]) => {
                matches!(*action, "replace" | "del") && matches!(*cidr, "::/1" | "8000::/1")
            }
            (Program::Ip, [family, "route", "add", cidr, "dev", iface, tail @ ..]) => {
                let network = cidr.parse::<ipnet::IpNet>();
                interface(iface)
                    && (tail.is_empty() || tail == ["table", "51820"])
                    && network.is_ok_and(|n| {
                        if *family == "-4" {
                            n.addr().is_ipv4()
                        } else {
                            *family == "-6" && n.addr().is_ipv6()
                        }
                    })
            }
            (Program::Ip, [family, "route", "show", "table", table, "exact", cidr]) => {
                matches!(*family, "-4" | "-6")
                    && matches!(*table, "main" | "51820")
                    && cidr
                        .parse::<ipnet::IpNet>()
                        .is_ok_and(|n| n.addr().is_ipv4() == (*family == "-4"))
            }
            (Program::Resolvectl, ["revert", iface]) => interface(iface),
            (Program::Resolvectl, ["dns", iface, servers @ ..]) => {
                interface(iface)
                    && !servers.is_empty()
                    && servers
                        .iter()
                        .all(|s| s.parse::<std::net::IpAddr>().is_ok())
            }
            (Program::Resolvectl, ["domain", iface, domains @ ..]) => {
                interface(iface) && domains.iter().all(|s| domain(s))
            }
            (Program::Resolvectl, ["default-route", iface, "yes"]) => interface(iface),
            (Program::Sysctl, ["-q", "-w", "net.ipv4.conf.all.src_valid_mark=1"])
            | (Program::Sysctl, ["-n", "net.ipv4.conf.all.src_valid_mark"]) => true,
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(NetError::InvalidConfig("command outside the network plan"))
        }
    }
    pub(crate) fn validate_undo(&self) -> Result<(), NetError> {
        self.validate()?;
        let args = self.args.iter().map(String::as_str).collect::<Vec<_>>();
        match (self.program, args.as_slice()) {
            (Program::Resolvectl, ["revert", _])
            | (Program::Ip, ["link", "del", _])
            | (Program::Ip, [_, "rule", "del", ..])
            | (Program::Ip, ["-6", "route", "del", "unreachable", _]) => Ok(()),
            _ => Err(NetError::InvalidConfig("journal contains an apply command")),
        }
    }
}
