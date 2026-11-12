#![forbid(unsafe_code)]
//! Test utilities that refuse to run outside the disposable Docker client.
use galleria_ipc::{State, Status, client::Client};
use std::{
    path::Path,
    process::{Command, Output},
    time::Duration,
};
pub fn assert_lab() {
    assert!(
        std::env::var("GALLERIA_LAB").is_ok_and(|v| v == "1") && Path::new("/.dockerenv").exists(),
        "end-to-end helpers require lab/run.sh"
    );
}
pub fn sh(cmd: &str) -> Output {
    assert_lab();
    Command::new("bash")
        .args(["-c", cmd])
        .output()
        .unwrap_or_else(|e| panic!("cannot run lab command: {e}"))
}
pub fn checked(cmd: &str) -> String {
    let out = sh(cmd);
    assert!(
        out.status.success(),
        "command failed: {cmd}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}
pub async fn client() -> Client {
    assert_lab();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(mut c) = Client::connect(Path::new(galleria_ipc::DEFAULT_SOCKET_PATH)).await
                && c.hello("e2e", "1").await.is_ok()
            {
                return c;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("daemon did not listen within 5s"))
}
pub async fn wait_state(c: &mut Client, wanted: State, timeout: Duration) -> Status {
    tokio::time::timeout(timeout, async {
        loop {
            let s: Status = c
                .call("status", &serde_json::json!({}))
                .await
                .unwrap_or_else(|e| panic!("status: {e}"));
            if s.state == wanted {
                return s;
            }
            assert!(
                s.state != State::Error || wanted == State::Error,
                "daemon error: {:?}",
                s.error
            );
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("did not reach {wanted:?}"))
}
pub fn residue() -> String {
    checked("ip -br link; ip route; ip -6 route; ip rule; ip -6 rule; resolvectl dns")
}
pub fn lab_file(name: &str) -> String {
    assert_lab();
    assert!(!name.contains('/') && !name.contains(".."));
    std::fs::read_to_string(Path::new("/lab/lab/out").join(name))
        .unwrap_or_else(|e| panic!("lab fixture unavailable: {e}"))
}

pub fn wg_params(name: &str) -> galleria_ipc::ConnectParams {
    use galleria_ipc::*;
    let text = lab_file(name);
    let mut private = None;
    let mut addresses = vec![];
    let mut dns = vec![];
    let mut public = String::new();
    let mut endpoint = None;
    let mut allowed = vec![];
    let mut keepalive = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let list = || {
            value
                .split(',')
                .map(|s| s.trim().to_owned())
                .collect::<Vec<_>>()
        };
        match key.trim() {
            "PrivateKey" => private = Some(Secret::new(value)),
            "Address" => addresses = list(),
            "DNS" => dns = list(),
            "PublicKey" => public = value.into(),
            "Endpoint" => endpoint = Some(value.into()),
            "AllowedIPs" => allowed = list(),
            "PersistentKeepalive" => keepalive = value.parse().ok(),
            _ => {}
        }
    }
    ConnectParams {
        profile: ProfileRef {
            id: "lab-wireguard".into(),
            name: "Lab WireGuard".into(),
        },
        replace: false,
        backend: BackendParams::Wireguard(WireGuardParams {
            private_key: private.unwrap_or_else(|| panic!("missing lab private key")),
            addresses,
            dns,
            search_domains: vec![],
            mtu: Some(1420),
            peers: vec![WireGuardPeer {
                public_key: public,
                preshared_key: None,
                endpoint,
                allowed_ips: allowed,
                persistent_keepalive: keepalive,
            }],
        }),
        options: ConnectOptions {
            ipv6_block: true,
            ignore_pushed_routes: false,
            reconnect: ReconnectOptions { enabled: false },
        },
    }
}
pub async fn disconnect(c: &mut Client) {
    let _: serde_json::Value = c
        .call("disconnect", &serde_json::json!({}))
        .await
        .unwrap_or_else(|e| panic!("disconnect: {e}"));
    wait_state(c, State::Disconnected, Duration::from_secs(5)).await;
}
pub async fn connect_wg(c: &mut Client, name: &str) -> Status {
    disconnect(c).await;
    let _: galleria_ipc::ConnectResult = c
        .call("connect", &wg_params(name))
        .await
        .unwrap_or_else(|e| panic!("connect: {e}"));
    wait_state(c, State::Connected, Duration::from_secs(5)).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    Ipv4,
    Ipv6,
}
/// Count global IPv6 on the physical link while generating real IPv6 HTTP attempts.
pub fn count_global_ipv6_on_eth0(secs: u64) -> u64 {
    use std::process::Stdio;
    assert_lab();
    assert!(secs > 0);
    let duration = secs.to_string();
    let capture = Command::new("timeout")
        .args([
            "--signal=INT",
            &duration,
            "tcpdump",
            "-ln",
            "-i",
            "eth0",
            "ip6 and not net fe80::/10 and not multicast",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("capture: {e}"));
    let mut traffic=Command::new("timeout").args([&duration,"bash","-c","while :; do curl --noproxy '*' -6 -s --max-time 1 'http://[2001:db8:99::1]/' >/dev/null; sleep .1; done"])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap_or_else(|e|panic!("traffic: {e}"));
    let output = capture
        .wait_with_output()
        .unwrap_or_else(|e| panic!("capture wait: {e}"));
    let _ = traffic.wait();
    assert_eq!(
        output.status.code(),
        Some(124),
        "capture did not run for the requested duration: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("listening on eth0"),
        "capture never started"
    );
    let packets = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .count() as u64;
    println!("global IPv6 packets on eth0 over {secs}s: {packets}");
    packets
}
/// Deliberately try AAAA before A, each with the application's 10-second timeout.
pub fn sequential_client(host: &str, port: u16) -> (Duration, AddressFamily) {
    assert_lab();
    let script = r#"
import socket, sys, time, json
addresses=socket.getaddrinfo(sys.argv[1],int(sys.argv[2]),type=socket.SOCK_STREAM)
assert any(a[0]==socket.AF_INET6 for a in addresses) and any(a[0]==socket.AF_INET for a in addresses)
addresses.sort(key=lambda a:a[0]!=socket.AF_INET6)
start=time.monotonic()
for family,_,_,_,address in addresses:
    try:
        with socket.create_connection((address[0],address[1]),timeout=10):
            print(json.dumps([time.monotonic()-start,6 if family==socket.AF_INET6 else 4]))
            break
    except OSError:
        pass
else:
    raise RuntimeError('all addresses failed')
"#;
    let output = Command::new("python3")
        .args(["-c", script, host, &port.to_string()])
        .output()
        .unwrap_or_else(|e| panic!("sequential client: {e}"));
    assert!(
        output.status.success(),
        "sequential client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let (seconds, family): (f64, u8) =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("sequential result: {e}"));
    println!("sequential client: IPv{family}, {seconds:.6}s");
    (
        Duration::from_secs_f64(seconds),
        match family {
            4 => AddressFamily::Ipv4,
            6 => AddressFamily::Ipv6,
            _ => panic!("invalid family"),
        },
    )
}

pub fn ovpn_params(profile: &str) -> galleria_ipc::OpenVpnParams {
    use galleria_ipc::*;
    let text = lab_file(&format!("{profile}.ovpn"));
    let items = galleria_ovpn::parse(&text).unwrap_or_else(|_| panic!("invalid lab profile"));
    let mut public = Vec::new();
    let mut secret_blocks = std::collections::BTreeMap::new();
    for item in items {
        match item {
            galleria_ovpn::Item::Block { tag, body, .. }
                if galleria_ovpn::SECRET_BLOCKS.contains(&tag.as_str()) =>
            {
                secret_blocks.insert(tag, Secret::new(body));
            }
            item => public.push(item),
        }
    }
    OpenVpnParams {
        config: galleria_ovpn::render(&public),
        secret_blocks,
        username: Some("alice".into()),
        password: Some(Secret::new("alice-secret")),
        otp: None,
        key_passphrase: None,
    }
}
