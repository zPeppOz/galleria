#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
use galleria_ipc::*;
use std::time::Duration;
fn request(profile: &str, extra: &str, block: bool) -> ConnectParams {
    let mut ovpn = ovpn_params(profile);
    ovpn.config.push_str(extra);
    ConnectParams {
        profile: ProfileRef {
            id: format!("v6-{profile}"),
            name: "IPv6 laboratory".into(),
        },
        replace: false,
        backend: BackendParams::Openvpn(ovpn),
        options: ConnectOptions {
            ipv6_block: block,
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
async fn connect(c: &mut client::Client, profile: &str, extra: &str, block: bool) -> Status {
    let _: ConnectResult = c
        .call("connect", &request(profile, extra, block))
        .await
        .unwrap();
    wait_state(c, State::Connected, Duration::from_secs(15)).await
}
fn blocked(s: Status) {
    let net = s.net.unwrap();
    assert_eq!(net.ipv6_block, Ipv6BlockInfo::Unreachable);
    assert!(net.full_tunnel);
    assert_eq!(count_global_ipv6_on_eth0(15), 0);
    let (time, family) = sequential_client("dual.galleria.test", 80);
    assert_eq!(family, AddressFamily::Ipv4);
    assert!(time < Duration::from_secs(2));
}
const ADDRESS_ONLY: &str = "pull-filter ignore \"redirect-gateway\"\nredirect-gateway def1\n";
#[tokio::test]
#[ignore]
async fn control_without_block_leaks_ipv6() {
    let mut c = clean().await;
    connect(&mut c, "base", "", false).await;
    assert!(count_global_ipv6_on_eth0(15) > 0);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn pushed_redirect_gets_unreachable_block() {
    let mut c = clean().await;
    blocked(connect(&mut c, "base", "", true).await);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn profile_redirect_gets_unreachable_block() {
    let mut c = clean().await;
    blocked(connect(&mut c, "split", "redirect-gateway def1\n", true).await);
    let logs: LogsResult = c
        .call(
            "logs",
            &LogsParams {
                after: None,
                limit: 1000,
            },
        )
        .await
        .unwrap();
    assert!(
        logs.entries
            .iter()
            .all(|l| !l.message.contains("block-ipv6") && !l.message.contains("ifconfig-ipv6"))
    );
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn split_tunnel_leaves_ipv6_untouched() {
    let mut c = clean().await;
    let s = connect(&mut c, "split", "", true).await;
    let net = s.net.unwrap();
    assert_eq!(net.ipv6_block, Ipv6BlockInfo::None);
    assert!(!net.full_tunnel);
    assert!(!checked("ip -6 route").contains("unreachable"));
    assert_eq!(
        checked(
            "curl --noproxy '*' -6 -s -o /dev/null -w '%{http_code}' --max-time 3 'http://[2001:db8:99::1]/'"
        ),
        "200"
    );
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn server_with_ipv6_default_keeps_ipv6_inside_the_tunnel() {
    let mut c = clean().await;
    let s = connect(&mut c, "v6", "", true).await;
    assert_eq!(s.net.unwrap().ipv6_block, Ipv6BlockInfo::None);
    let logs: LogsResult = c
        .call(
            "logs",
            &LogsParams {
                after: None,
                limit: 1000,
            },
        )
        .await
        .unwrap();
    let source = logs
        .entries
        .iter()
        .find(|l| l.message.starts_with("IPv6 default sources at up:"))
        .expect("missing source evidence");
    println!("{}", source.message);
    assert!(source.message.contains("push=true") || source.message.contains("env=true"));
    assert!(!checked("ip -6 route").contains("unreachable"));
    assert!(checked("ip -6 route get 2001:db8:99::1").contains("dev glr-ovpn0"));
    assert_eq!(
        checked(
            "curl --noproxy '*' -6 -s -o /dev/null -w '%{http_code}' --max-time 3 'http://[2001:db8:99::1]/'"
        ),
        "200"
    );
    assert_eq!(count_global_ipv6_on_eth0(15), 0);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn ipv6_address_without_ipv6_default_keeps_the_block() {
    let mut c = clean().await;
    let s = connect(&mut c, "v6", ADDRESS_ONLY, true).await;
    assert!(checked("ip -6 addr show glr-ovpn0 scope global").contains("2001:db8:88:"));
    blocked(s);
    checked("ping -6 -c 2 -W 2 2001:db8:88::1");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn control_ipv6_address_without_block_leaks() {
    let mut c = clean().await;
    connect(&mut c, "v6", ADDRESS_ONLY, false).await;
    assert!(checked("ip -6 addr show glr-ovpn0 scope global").contains("2001:db8:88:"));
    assert!(count_global_ipv6_on_eth0(15) > 0);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn unreachable_routes_are_gone_after_disconnect() {
    let mut c = clean().await;
    let before = residue();
    connect(&mut c, "base", "", true).await;
    assert!(checked("ip -6 route").contains("unreachable"));
    disconnect(&mut c).await;
    assert_eq!(residue(), before);
    assert!(!checked("ip -6 route").contains("unreachable"));
}
#[tokio::test]
#[ignore]
async fn unreachable_routes_are_gone_after_kill9() {
    let mut c = clean().await;
    let before = residue();
    connect(&mut c, "base", "", true).await;
    assert!(checked("ip -6 route").contains("unreachable"));
    let pid = checked("systemctl show galleriad -p MainPID --value");
    checked("systemctl kill -s KILL --kill-whom=main galleriad");
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let new = checked("systemctl show galleriad -p MainPID --value");
            if new != "0" && new != pid {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let mut c = client().await;
    let h = c.hello("ipv6-test", "1").await.unwrap();
    assert_eq!(h.status.state, State::Disconnected);
    assert_eq!(residue(), before);
    assert!(!checked("ip -6 route").contains("unreachable"));
}
