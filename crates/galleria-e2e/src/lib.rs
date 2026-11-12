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
