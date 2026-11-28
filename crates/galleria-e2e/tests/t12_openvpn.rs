#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
use galleria_ipc::*;
use std::time::Duration;
fn request(profile: &str) -> ConnectParams {
    ConnectParams {
        profile: ProfileRef {
            id: format!("ovpn-{profile}"),
            name: "Lab OpenVPN".into(),
        },
        replace: false,
        backend: BackendParams::Openvpn(ovpn_params(profile)),
        options: ConnectOptions {
            ipv6_block: true,
            ignore_pushed_routes: false,
            reconnect: ReconnectOptions { enabled: false },
        },
    }
}
async fn clean() -> client::Client {
    let mut c = client().await;
    disconnect(&mut c).await;
    c
}
async fn connect(c: &mut client::Client, profile: &str) -> Status {
    let _: ConnectResult = c.call("connect", &request(profile)).await.unwrap();
    wait_state(c, State::Connected, Duration::from_secs(15)).await
}
async fn subscribed() -> client::Client {
    let mut c = clean().await;
    let _: serde_json::Value = c
        .call(
            "subscribe",
            &SubscribeParams {
                stats: true,
                logs: true,
            },
        )
        .await
        .unwrap();
    c
}
#[tokio::test]
#[ignore]
async fn hello_reports_openvpn_version() {
    let mut c = clean().await;
    let h = c.hello("test", "1").await.unwrap();
    let b = h
        .backends
        .iter()
        .find(|b| b.protocol == Protocol::Openvpn)
        .unwrap();
    assert!(b.available);
    let version = checked("openvpn --version | head -1");
    let version = version.split_whitespace().nth(1).unwrap();
    let prefix = version.rsplit_once('.').unwrap().0;
    assert!(b.version.as_ref().unwrap().starts_with(prefix));
}
#[tokio::test]
#[ignore]
async fn ovpn_connects_with_password() {
    let mut c = clean().await;
    let s = connect(&mut c, "base").await;
    assert!(s.local_addresses[0].to_string().starts_with("10.8.0."));
    checked("ping -c 3 -W 2 10.8.0.1");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn ovpn_reports_progress_before_connected() {
    let mut c = subscribed().await;
    let _: ConnectResult = c.call("connect", &request("base")).await.unwrap();
    let progress = tokio::time::timeout(Duration::from_secs(15), async {
        let mut progress = false;
        loop {
            if let Some(Event::State(s)) = c.next_event().await {
                if s.state == State::Connecting
                    && s.detail
                        .as_deref()
                        .is_some_and(|s| ["AUTH", "GET_CONFIG", "ASSIGN_IP"].contains(&s))
                {
                    progress = true;
                }
                if s.state == State::Connected {
                    break progress;
                }
                assert_ne!(s.state, State::Error, "{:?}", s.error);
            }
        }
    })
    .await
    .unwrap();
    assert!(progress);
    disconnect(&mut c).await;
}
async fn stats(c: &mut client::Client) -> Stats {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(Event::Stats(s)) = c.next_event().await {
                return s;
            }
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
#[ignore]
async fn ovpn_stats_grow_during_ping() {
    let mut c = subscribed().await;
    connect(&mut c, "base").await;
    let first = stats(&mut c).await;
    checked("ping -c 4 -i .3 -W 2 10.8.0.1");
    tokio::time::timeout(Duration::from_secs(5), async {
        while stats(&mut c).await.rx_bytes <= first.rx_bytes {}
    })
    .await
    .unwrap();
    disconnect(&mut c).await;
}
async fn dns(profile: &str) {
    let mut c = subscribed().await;
    let _: ConnectResult = c.call("connect", &request(profile)).await.unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Some(Event::State(s)) = c.next_event().await {
                assert_ne!(s.state, State::Error, "{:?}", s.error);
                if s.state == State::Connected {
                    assert!(checked("resolvectl dns glr-ovpn0").contains(
                        if profile == "dns-option" {
                            "10.9.0.1"
                        } else {
                            "10.8.0.1"
                        }
                    ));
                    assert!(
                        checked("resolvectl query intranet.galleria.test").contains("glr-ovpn0")
                    );
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn ovpn_pushed_dns_is_applied_before_connected() {
    dns("base").await;
}
#[tokio::test]
#[ignore]
async fn ovpn_dns_option_server_works() {
    dns("dns-option").await;
}
#[tokio::test]
#[ignore]
async fn status_shows_pushed_routes_and_dns() {
    let mut c = clean().await;
    let s = connect(&mut c, "base").await;
    let n = s.net.unwrap();
    assert!(n.full_tunnel);
    assert!(n.routes.contains(&"192.0.2.0/24".into()));
    assert_eq!(n.dns_servers[0].to_string(), "10.8.0.1");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn ignore_pushed_routes_drops_server_routes() {
    let mut c = clean().await;
    let mut p = request("base");
    p.options.ignore_pushed_routes = true;
    let _: ConnectResult = c.call("connect", &p).await.unwrap();
    let s = wait_state(&mut c, State::Connected, Duration::from_secs(15)).await;
    assert!(!s.net.unwrap().full_tunnel);
    assert!(!checked("ip route").contains("192.0.2.0/24"));
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn ovpn_cmdline_and_disk_have_no_secrets() {
    let mut c = clean().await;
    connect(&mut c, "base").await;
    checked(
        "python3 - <<'PY'\nfrom pathlib import Path\nimport subprocess,os\npid=subprocess.check_output(['pgrep','-xo','openvpn']).decode().strip()\nargs=Path('/proc/'+pid+'/cmdline').read_bytes().split(b'\\0')[:-1]\nassert len(args)==3 and args[0].endswith(b'/openvpn') and args[1:]==[b'--config',b'/dev/stdin']\nprofile=Path('/lab/lab/out/base.ovpn').read_text()\nkey=profile.split('<key>')[1].split('</key>')[0]\nneedle=next(l.encode() for l in key.splitlines() if len(l)>40 and not l.startswith('-'))\nleaks=[]\nfor root in ['/run','/tmp','/var','/etc']:\n for d,dirs,files in os.walk(root):\n  for name in files:\n   p=Path(d,name)\n   try:\n    if not p.is_symlink() and p.is_file() and needle in p.read_bytes(): leaks.append(str(p))\n   except (OSError,PermissionError): pass\nassert not leaks, leaks\nPY",
    );
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn mgmt_socket_is_denied_to_normal_user() {
    let mut c = clean().await;
    connect(&mut c, "base").await;
    let path = checked("find /run/galleria/s -name mgmt.sock -print");
    assert_eq!(path.lines().count(), 1);
    let out = sh(&format!(
        "runuser -u alice -- python3 -c 'import socket; socket.socket(socket.AF_UNIX).connect(\"{path}\")'"
    ));
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Permission denied"));
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn malicious_profile_is_rejected_and_nothing_runs() {
    let mut c = clean().await;
    let e = c
        .call::<_, ConnectResult>("connect", &request("malicious"))
        .await
        .unwrap_err();
    assert!(
        matches!(e,client::ClientError::Rpc(e) if e.data.as_ref().unwrap()["code"]=="profile_rejected")
    );
    assert!(checked("find /tmp -name 'PWNED-*' -print").is_empty());
    assert!(!sh("pgrep -x openvpn").status.success());
}
#[tokio::test]
#[ignore]
async fn ovpn_disconnect_leaves_no_residue() {
    let mut c = clean().await;
    let before = residue();
    connect(&mut c, "base").await;
    disconnect(&mut c).await;
    assert_eq!(residue(), before);
    assert!(checked("find /run/galleria/s -mindepth 1 -print").is_empty());
}
#[tokio::test]
#[ignore]
async fn session_dir_name_is_random_and_private() {
    let mut c = clean().await;
    let mut names = vec![];
    for _ in 0..2 {
        let s = connect(&mut c, "base").await;
        let name = checked("find /run/galleria/s -mindepth 1 -maxdepth 1 -printf '%f\\n'");
        assert_eq!(name.lines().count(), 1);
        assert_ne!(name, s.session.unwrap().to_string());
        assert_eq!(checked("stat -c '%a:%u' /run/galleria/s/*"), "700:0");
        names.push(name);
        disconnect(&mut c).await;
    }
    assert_ne!(names[0], names[1]);
}
#[tokio::test]
#[ignore]
async fn kill9_daemon_kills_openvpn_and_cleans() {
    let mut c = clean().await;
    let before = residue();
    connect(&mut c, "base").await;
    let old = checked("systemctl show galleriad -p MainPID --value");
    checked("systemctl kill -s KILL --kill-whom=main galleriad");
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let pid = checked("systemctl show galleriad -p MainPID --value");
            if pid != "0" && pid != old {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let mut c = client().await;
    let h = c.hello("test", "1").await.unwrap();
    assert_eq!(h.status.state, State::Disconnected);
    assert!(!sh("pgrep -x openvpn").status.success());
    assert_eq!(residue(), before);
    assert!(checked("find /run/galleria/s -mindepth 1 -print").is_empty());
}
#[tokio::test]
#[ignore]
async fn replace_wireguard_with_openvpn() {
    let mut c = clean().await;
    connect_wg(&mut c, "wg-split.conf").await;
    let mut p = request("base");
    p.replace = true;
    let next: ConnectResult = c.call("connect", &p).await.unwrap();
    let s = wait_state(&mut c, State::Connected, Duration::from_secs(15)).await;
    assert_eq!(s.session, Some(next.session));
    assert!(!sh("ip link show glr-wg0").status.success());
    checked("ip link show glr-ovpn0");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn ovpn_dco_state_and_bytecount() {
    if std::env::var("GALLERIA_E2E_DCO").as_deref() != Ok("1") {
        eprintln!("DCO skipped: requires GALLERIA_E2E_DCO=1");
        return;
    }
    let mut c = subscribed().await;
    connect(&mut c, "base").await;
    checked("ping -c 3 -W 2 10.8.0.1");
    assert!(stats(&mut c).await.rx_bytes > 0);
    // The daemon serves at most 1000 lines per call, oldest first: page with `after`.
    let mut after = None;
    let mut opened = false;
    loop {
        let logs: LogsResult = c
            .call("logs", &LogsParams { after, limit: 1000 })
            .await
            .unwrap();
        opened |= logs
            .entries
            .iter()
            .any(|l| l.message.contains("DCO device") && l.message.contains("opened"));
        if opened || logs.entries.len() < 1000 {
            break;
        }
        after = Some(logs.last);
    }
    assert!(opened, "no 'DCO device ... opened' line in the daemon logs");
    disconnect(&mut c).await;
}
