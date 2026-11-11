use ipnet::IpNet;
use std::net::IpAddr;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ipv6Block {
    None,
    Unreachable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetConfig {
    pub interface: String,
    pub dns_servers: Vec<IpAddr>,
    pub search_domains: Vec<String>,
    pub routes: Vec<IpNet>,
    pub install_routes: Option<u32>,
    pub ipv6_block: Ipv6Block,
}
pub fn is_full_tunnel_v4(routes: &[IpNet]) -> bool {
    let mut lower = false;
    let mut upper = false;
    for route in routes {
        if let IpNet::V4(net) = route {
            match net.prefix_len() {
                0 => return true,
                1 => {
                    if net.network().is_unspecified() {
                        lower = true
                    } else {
                        upper = true
                    }
                }
                _ => {}
            }
        }
    }
    lower && upper
}
pub fn carries_ipv6_default(routes: &[IpNet]) -> bool {
    // Merge intervals over the global unicast range 2000::/3, including overlaps and adjacent subnets.
    let first = 0x2000_u128 << 112;
    let last = (0x4000_u128 << 112) - 1;
    let mut intervals = routes
        .iter()
        .filter_map(|route| {
            if let IpNet::V6(net) = route {
                Some((u128::from(net.network()), u128::from(net.broadcast())))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    intervals.sort_unstable();
    let mut next = first;
    for (start, end) in intervals {
        if end < next {
            continue;
        }
        if start > next {
            return false;
        }
        if end >= last {
            return true;
        }
        next = end + 1;
    }
    false
}
