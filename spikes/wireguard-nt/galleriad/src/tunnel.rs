//! The WireGuard tunnel: WireGuardNT adapter, addresses and routes, counters, IPv6 block.

use std::net::SocketAddr;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use ipnet::IpNet;
use serde_json::{json, Value};
use wireguard_nt::{Adapter, SetInterface, SetPeer};

use crate::win::log;

const ADAPTER: &str = "galleria-wg";
const FW_RULE: &str = "galleria-spike-block-ipv6";

/// How IPv6 is kept out of the physical interface when the profile is IPv4 only (criterion 1.5).
#[derive(serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Ipv6Block {
    #[default]
    None,
    /// D15 as written: ::/1 and 8000::/1 towards the tunnel, where no peer accepts them
    Route,
    /// Windows Filtering Platform through the firewall: outbound block of global IPv6
    Firewall,
}

struct Tunnel {
    adapter: Adapter,
    ipv6: Ipv6Block,
    since: SystemTime,
}

static TUNNEL: Mutex<Option<Tunnel>> = Mutex::new(None);

struct Conf {
    private_key: [u8; 32],
    addresses: Vec<IpNet>,
    peers: Vec<SetPeer>,
}

fn key(s: &str) -> Result<[u8; 32], String> {
    let v = base64::engine::general_purpose::STANDARD.decode(s.trim()).map_err(|e| format!("bad key: {e}"))?;
    v.try_into().map_err(|_| "bad key length".to_string())
}

/// Minimal wg-quick style parser: [Interface] PrivateKey, Address; [Peer] PublicKey, PresharedKey,
/// Endpoint (ip:port), AllowedIPs, PersistentKeepalive. Everything else is refused, not ignored.
fn parse(text: &str) -> Result<Conf, String> {
    let mut private_key = None;
    let mut addresses = Vec::new();
    let mut peers: Vec<SetPeer> = Vec::new();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = line.to_ascii_lowercase();
            if section == "[peer]" {
                peers.push(SetPeer { public_key: None, preshared_key: None, keep_alive: None, endpoint: "0.0.0.0:0".parse().unwrap(), allowed_ips: Vec::new() });
            }
            continue;
        }
        let (k, v) = line.split_once('=').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim())).ok_or("bad line")?;
        match (section.as_str(), k.as_str()) {
            ("[interface]", "privatekey") => private_key = Some(key(v)?),
            ("[interface]", "address") => {
                for a in v.split(',') {
                    addresses.push(a.trim().parse::<IpNet>().map_err(|e| format!("Address: {e}"))?);
                }
            }
            ("[peer]", k) => {
                let p = peers.last_mut().unwrap();
                match k {
                    "publickey" => p.public_key = Some(key(v)?),
                    "presharedkey" => p.preshared_key = Some(key(v)?),
                    "endpoint" => p.endpoint = v.parse::<SocketAddr>().map_err(|e| format!("Endpoint: {e}"))?,
                    "persistentkeepalive" => p.keep_alive = Some(v.parse().map_err(|_| "PersistentKeepalive")?),
                    "allowedips" => {
                        for a in v.split(',') {
                            p.allowed_ips.push(a.trim().parse::<IpNet>().map_err(|e| format!("AllowedIPs: {e}"))?);
                        }
                    }
                    other => return Err(format!("directive not allowed in [Peer]: {other}")),
                }
            }
            (s, other) => return Err(format!("directive not allowed in {s}: {other}")),
        }
    }
    Ok(Conf { private_key: private_key.ok_or("PrivateKey missing")?, addresses, peers })
}

fn run(cmd: &[&str]) -> bool {
    let out = Command::new(cmd[0]).args(&cmd[1..]).output();
    let (ok, text) = match out {
        Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stdout).trim().to_string()),
        Err(e) => (false, e.to_string()),
    };
    log(&format!("net: {} -> ok={ok} {}", cmd.join(" "), text.replace(['\r', '\n'], " ")));
    ok
}

pub fn connect(config: &str, ipv6: Ipv6Block) -> Result<Value, String> {
    let mut guard = TUNNEL.lock().unwrap();
    if guard.is_some() {
        return Err("already connected".into());
    }
    let conf = parse(config)?;
    // wireguard.dll sits next to the service binary: full path, never the DLL search path
    let dll = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("wireguard.dll");
    let wg = unsafe { wireguard_nt::load_from_path(&dll) }.map_err(|e| format!("load {}: {e}", dll.display()))?;
    log(&format!("wg: loaded {}", dll.display()));
    let adapter = Adapter::create(&wg, "Galleria", ADAPTER, None).map_err(|e| format!("WireGuardCreateAdapter: {e}"))?;
    log(&format!("wg: adapter `{ADAPTER}` created, driver version {:?}", wireguard_nt::get_running_driver_version(&wg.clone().into_inner())));
    let iface = SetInterface { listen_port: None, public_key: None, private_key: Some(conf.private_key), peers: conf.peers };
    adapter.set_config(&iface).map_err(|e| format!("WireGuardSetConfiguration: {e}"))?;
    adapter.set_default_route(&conf.addresses, &iface).map_err(|e| format!("addresses/routes: {e}"))?;
    adapter.up().map_err(|e| format!("WireGuardSetAdapterState(up): {e}"))?;
    log(&format!("wg: adapter up, addresses {:?}, {} peer(s), ipv6 block {ipv6:?}", conf.addresses, iface.peers.len()));
    match ipv6 {
        Ipv6Block::None => {}
        Ipv6Block::Route => {
            run(&["netsh", "interface", "ipv6", "add", "route", "::/1", &format!("interface={ADAPTER}"), "store=active"]);
            run(&["netsh", "interface", "ipv6", "add", "route", "8000::/1", &format!("interface={ADAPTER}"), "store=active"]);
        }
        Ipv6Block::Firewall => {
            // spike shortcut: a firewall rule (it is a WFP filter underneath). The real daemon would add
            // the filter itself in a dynamic WFP session, so that it disappears if the daemon dies.
            run(&["netsh", "advfirewall", "firewall", "add", "rule", &format!("name={FW_RULE}"), "dir=out", "action=block", "remoteip=2000::/3,fc00::/7"]);
        }
    }
    *guard = Some(Tunnel { adapter, ipv6, since: SystemTime::now() });
    Ok(json!({"adapter": ADAPTER, "ipv6_block": format!("{ipv6:?}")}))
}

/// Closing the adapter handle removes the adapter and, with it, its addresses and routes.
pub fn disconnect() -> bool {
    let t = TUNNEL.lock().unwrap().take();
    match t {
        Some(t) => {
            if t.ipv6 == Ipv6Block::Firewall {
                run(&["netsh", "advfirewall", "firewall", "delete", "rule", &format!("name={FW_RULE}")]);
            }
            let _ = t.adapter.down();
            drop(t.adapter);
            log("wg: adapter closed");
            true
        }
        None => false,
    }
}

fn snapshot(t: &Tunnel) -> Value {
    let c = t.adapter.get_config();
    let peers: Vec<Value> = c
        .peers
        .iter()
        .map(|p| {
            let hs = p.last_handshake.and_then(|h| h.duration_since(SystemTime::UNIX_EPOCH).ok()).map(|d| d.as_secs());
            let age = p.last_handshake.and_then(|h| SystemTime::now().duration_since(h).ok()).map(|d| d.as_secs());
            json!({"endpoint": p.endpoint.to_string(), "last_handshake_unix": hs, "last_handshake_age_secs": age, "rx_bytes": p.rx_bytes, "tx_bytes": p.tx_bytes})
        })
        .collect();
    json!({"connected": true, "adapter": ADAPTER, "up_secs": t.since.elapsed().map(|d| d.as_secs()).unwrap_or(0), "peers": peers})
}

pub fn status() -> Value {
    match TUNNEL.lock().unwrap().as_ref() {
        Some(t) => snapshot(t),
        None => json!({"connected": false}),
    }
}

/// Criterion 1.2: last handshake and byte counters, read every second while a tunnel is up.
pub fn start_stats_thread() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(1));
        if let Some(t) = TUNNEL.lock().unwrap().as_ref() {
            log(&format!("stats: {}", snapshot(t)["peers"]));
        }
    });
}
