//! Common network module (D10) and the fixed `--up`/`--down` helper.
//!
//! openvpn runs `galleriad-spike netcfg <socket>` as up/down command. The helper
//! forwards the script environment (foreign_option_*, dns_*, route_*, ...) to the
//! daemon over a root-only Unix socket and exits with the daemon's verdict.
//! The daemon then applies DNS (systemd-resolved) and, optionally, the IPv6 block.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};

use crate::{emit, Ipv6Mode};
use serde_json::json;

/// Entry point of the helper (child of openvpn).
pub fn helper_main(sock: &str) -> i32 {
    let env: BTreeMap<String, String> = std::env::vars()
        .filter(|(k, _)| !k.contains("password") && !k.starts_with("LS_"))
        .collect();
    let (r, mut w) = match crate::plat::netcfg_client(sock) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("galleria netcfg: cannot reach daemon: {e}");
            return 1;
        }
    };
    let mut msg = serde_json::to_string(&env).unwrap();
    msg.push('\n');
    if w.write_all(msg.as_bytes()).is_err() {
        return 1;
    }
    let _ = w.flush();
    let mut reply = String::new();
    let _ = BufReader::new(r).read_line(&mut reply);
    if reply.trim() == "ok" { 0 } else { 1 }
}

/// What the backend hands to the common module, protocol independent.
#[derive(Debug, Default, serde::Serialize)]
pub struct NetConfig {
    pub dev: String,
    pub dns_servers: Vec<String>,
    pub domains: Vec<String>,
    pub redirect_gateway: bool,
}

/// Extract DNS/domains from openvpn env. Two channels exist:
/// - `foreign_option_N=dhcp-option DNS x` (classic, still exported by 2.6 and 2.7?)
/// - `dns_server_N_address_M=x`, `dns_search_domain_N=d` (the `--dns` option, 2.6+)
pub fn parse_env(env: &BTreeMap<String, String>) -> (NetConfig, Vec<&'static str>) {
    let mut c = NetConfig { dev: env.get("dev").cloned().unwrap_or_default(), ..Default::default() };
    let mut channels = Vec::new();
    for (k, v) in env {
        if k.starts_with("foreign_option_") {
            let t: Vec<&str> = v.split_whitespace().collect();
            match t.as_slice() {
                ["dhcp-option", "DNS", ip] | ["dhcp-option", "DNS6", ip] => {
                    push_unique(&mut c.dns_servers, ip);
                    if !channels.contains(&"foreign_option") { channels.push("foreign_option"); }
                }
                ["dhcp-option", "DOMAIN", d] | ["dhcp-option", "DOMAIN-SEARCH", d] => push_unique(&mut c.domains, d),
                _ => {}
            }
        } else if k.starts_with("dns_server_") && k.contains("_address_") {
            push_unique(&mut c.dns_servers, v);
            if !channels.contains(&"dns_server") { channels.push("dns_server"); }
        } else if k.starts_with("dns_search_domain_") {
            push_unique(&mut c.domains, v);
        }
    }
    c.redirect_gateway = env.get("redirect_gateway").is_some_and(|v| v != "0");
    (c, channels)
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    // dns_server_N_address_M may carry "ip" or "ip:port"; keep as is for the spike
    if !v.iter().any(|x| x == s) {
        v.push(s.to_string());
    }
}

#[cfg(unix)]
fn run(cmd: &[&str]) -> bool {
    let out = std::process::Command::new(cmd[0]).args(&cmd[1..]).output();
    let (ok, err) = match out {
        Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => (false, e.to_string()),
    };
    emit("netcfg_cmd", json!({"cmd": cmd.join(" "), "ok": ok, "stderr": err}));
    ok
}

/// Windows: the spike only records what openvpn hands over; what openvpn applies by itself is
/// observed from outside (criterion 2.7, Windows side).
#[cfg(windows)]
pub fn apply_up(_c: &NetConfig, _ipv6: Ipv6Mode) -> bool {
    emit("netcfg_cmd", json!({"cmd": "(none)", "ok": true, "stderr": "windows: nothing applied by the daemon in this spike"}));
    true
}

#[cfg(windows)]
pub fn apply_down(_c: &NetConfig, _ipv6: Ipv6Mode) -> bool {
    true
}

#[cfg(unix)]
pub fn apply_up(c: &NetConfig, ipv6: Ipv6Mode) -> bool {
    let mut ok = true;
    if !c.dns_servers.is_empty() {
        let mut a = vec!["resolvectl", "dns", c.dev.as_str()];
        a.extend(c.dns_servers.iter().map(|s| s.as_str()));
        ok &= run(&a);
        let mut d = vec!["resolvectl", "domain", c.dev.as_str()];
        d.extend(c.domains.iter().map(|s| s.as_str()));
        // everything through the tunnel: also route all queries to it
        d.push("~.");
        ok &= run(&d);
        ok &= run(&["resolvectl", "default-route", c.dev.as_str(), "yes"]);
    }
    if ipv6 == Ipv6Mode::Common {
        // D15: all global IPv6 towards the tunnel, more specific than the LAN default route
        ok &= run(&["ip", "-6", "route", "add", "::/1", "dev", c.dev.as_str()]);
        ok &= run(&["ip", "-6", "route", "add", "8000::/1", "dev", c.dev.as_str()]);
    }
    if ipv6 == Ipv6Mode::Reject {
        ok &= run(&["ip", "-6", "route", "add", "unreachable", "::/1"]);
        ok &= run(&["ip", "-6", "route", "add", "unreachable", "8000::/1"]);
    }
    ok
}

#[cfg(unix)]
pub fn apply_down(c: &NetConfig, ipv6: Ipv6Mode) -> bool {
    if ipv6 == Ipv6Mode::Reject {
        // not tied to the tun device: must be removed explicitly
        run(&["ip", "-6", "route", "del", "unreachable", "::/1"]);
        run(&["ip", "-6", "route", "del", "unreachable", "8000::/1"]);
    }
    // openvpn calls --down after closing the tun device: resolved already dropped the link state
    if std::path::Path::new(&format!("/sys/class/net/{}", c.dev)).exists() {
        run(&["resolvectl", "revert", c.dev.as_str()]);
    } else {
        emit("netcfg_cmd", json!({"cmd": "(none)", "ok": true, "stderr": format!("{} already gone, resolved state dropped with it", c.dev)}));
    }
    true
}

/// Daemon side: one helper connection (the platform module owns the accept loop).
pub fn handle(conn: impl Read, mut reply: impl Write, ipv6: Ipv6Mode) {
    let mut line = String::new();
    if BufReader::new(conn).read_line(&mut line).is_err() {
        return;
    }
    let env: BTreeMap<String, String> = serde_json::from_str(&line).unwrap_or_default();
    let script = env.get("script_type").cloned().unwrap_or_default();
    let interesting: BTreeMap<_, _> = env
        .iter()
        .filter(|(k, _)| {
            k.starts_with("foreign_option_") || k.starts_with("dns_") || k.starts_with("route_")
                || k.starts_with("ifconfig_") || k.starts_with("redirect") || *k == "dev"
                || *k == "script_type" || k.starts_with("tun_") || *k == "dev_type"
        })
        .collect();
    let (cfg, channels) = parse_env(&env);
    emit("netcfg_env", json!({"script_type": script, "env": interesting, "parsed": cfg, "dns_channels": channels}));
    let ok = match script.as_str() {
        "up" => apply_up(&cfg, ipv6),
        "down" => apply_down(&cfg, ipv6),
        _ => true,
    };
    let _ = reply.write_all(if ok { b"ok\n" } else { b"fail\n" });
    let _ = reply.flush();
}
