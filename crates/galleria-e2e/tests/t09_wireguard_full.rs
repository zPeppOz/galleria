#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
use galleria_ipc::*;
use std::time::{Duration, Instant};
async fn full(c: &mut client::Client, block: bool) -> Status {
    disconnect(c).await;
    let mut p = wg_params("wg-full.conf");
    p.options.ipv6_block = block;
    let _: ConnectResult = c.call("connect", &p).await.unwrap();
    wait_state(c, State::Connected, Duration::from_secs(5)).await
}
#[tokio::test]
#[ignore]
async fn control_without_block_leaks_ipv6() {
    let mut c = client().await;
    full(&mut c, false).await;
    assert!(count_global_ipv6_on_eth0(15) > 0);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn full_tunnel_routes_all_ipv4() {
    let mut c = client().await;
    full(&mut c, true).await;
    assert!(checked("ip route get 198.51.100.1").contains("dev glr-wg0"));
    assert!(checked("ip rule").contains("not from all fwmark 0xca6c lookup 51820"));
    checked("ping -c 2 -W 2 10.99.0.1");
    assert_eq!(
        checked(
            "curl --noproxy '*' --max-time 5 -4 -s -o /dev/null -w '%{http_code}' http://198.51.100.1/"
        ),
        "200"
    );
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn full_tunnel_blocks_ipv6_with_zero_packets() {
    let mut c = client().await;
    full(&mut c, false).await;
    assert!(
        count_global_ipv6_on_eth0(15) > 0,
        "control must observe global IPv6"
    );
    disconnect(&mut c).await;
    full(&mut c, true).await;
    assert_eq!(count_global_ipv6_on_eth0(15), 0);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn ipv6_connect_fails_immediately() {
    let mut c = client().await;
    full(&mut c, false).await;
    std::net::TcpStream::connect_timeout(
        &"[2001:db8:99::1]:80".parse().unwrap(),
        Duration::from_secs(2),
    )
    .unwrap();
    disconnect(&mut c).await;
    full(&mut c, true).await;
    let start = Instant::now();
    let result = std::net::TcpStream::connect_timeout(
        &"[2001:db8:99::1]:80".parse().unwrap(),
        Duration::from_secs(2),
    );
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_millis(200));
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn sequential_client_falls_back_to_ipv4_under_2s() {
    let mut c = client().await;
    full(&mut c, false).await;
    assert_eq!(
        sequential_client("dual.galleria.test", 80).1,
        AddressFamily::Ipv6
    );
    disconnect(&mut c).await;
    full(&mut c, true).await;
    let (elapsed, family) = sequential_client("dual.galleria.test", 80);
    assert_eq!(family, AddressFamily::Ipv4);
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn lan_ipv6_still_works_during_block() {
    let mut c = client().await;
    full(&mut c, true).await;
    checked("ping -6 -c 2 -W 2 2001:db8:31::20");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn status_reports_full_tunnel_and_unreachable_block() {
    let mut c = client().await;
    let s = full(&mut c, true).await;
    let net = s.net.unwrap();
    assert!(net.full_tunnel);
    assert_eq!(net.ipv6_block, Ipv6BlockInfo::Unreachable);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn disconnect_removes_rules_and_unreachable_routes() {
    let mut c = client().await;
    disconnect(&mut c).await;
    let before = residue();
    full(&mut c, true).await;
    disconnect(&mut c).await;
    assert_eq!(residue(), before);
    assert_eq!(
        checked("find /run/galleria/s -mindepth 1 -maxdepth 1 | wc -l"),
        "0"
    );
    assert!(!std::path::Path::new("/run/galleria/net-journal.json").exists());
}
#[tokio::test]
#[ignore]
async fn kill9_then_restart_cleans_everything() {
    let mut c = client().await;
    disconnect(&mut c).await;
    let before = residue();
    full(&mut c, true).await;
    assert!(std::path::Path::new("/run/galleria/net-journal.json").exists());
    assert_ne!(
        checked("find /run/galleria/s -mindepth 1 -maxdepth 1 | wc -l"),
        "0"
    );
    let pid = checked("systemctl show galleriad -p MainPID --value");
    checked("systemctl kill -s KILL galleriad");
    drop(c);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let next = checked("systemctl show galleriad -p MainPID --value");
            if next != "0" && next != pid {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    let mut c = client().await;
    assert_eq!(
        c.hello("after-crash", "1").await.unwrap().status.state,
        State::Disconnected
    );
    assert_eq!(residue(), before);
    assert_eq!(
        checked("find /run/galleria/s -mindepth 1 -maxdepth 1 | wc -l"),
        "0"
    );
    assert!(!std::path::Path::new("/run/galleria/net-journal.json").exists());
}

#[tokio::test]
#[ignore]
async fn dns_unavailable_does_not_block_recovery() {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = sh(
                "systemctl unmask systemd-resolved.service; systemctl start systemd-resolved.service; systemctl restart galleriad",
            );
        }
    }
    let mut c = client().await;
    disconnect(&mut c).await;
    let _restore = Restore;
    checked("systemctl mask --now systemd-resolved.service");
    let _: ConnectResult = c
        .call("connect", &wg_params("wg-split.conf"))
        .await
        .unwrap();
    let s = wait_state(&mut c, State::Error, Duration::from_secs(5)).await;
    assert_eq!(s.error.unwrap().code, SessionErrorCode::DnsUnavailable);
    assert!(!std::path::Path::new("/run/galleria/net-journal.json").exists());
    assert!(!std::path::Path::new("/sys/class/net/glr-wg0").exists());
    checked("systemctl restart galleriad");
    let mut c = client().await;
    assert_eq!(
        c.hello("dns-recovery", "1").await.unwrap().status.state,
        State::Disconnected
    );
}

#[tokio::test]
#[ignore]
async fn disconnect_without_dns_does_not_wait_for_resolved() {
    struct Resume;
    impl Drop for Resume {
        fn drop(&mut self) {
            let _ = sh("systemctl kill -s CONT systemd-resolved.service");
        }
    }
    let mut c = client().await;
    disconnect(&mut c).await;
    let mut p = wg_params("wg-split.conf");
    if let BackendParams::Wireguard(w) = &mut p.backend {
        w.dns.clear();
    }
    let _: ConnectResult = c.call("connect", &p).await.unwrap();
    wait_state(&mut c, State::Connected, Duration::from_secs(5)).await;
    let _resume = Resume;
    checked("systemctl kill -s STOP systemd-resolved.service");
    tokio::time::timeout(Duration::from_secs(4), disconnect(&mut c))
        .await
        .expect("device removal waited for DNS despite no profile DNS");
    assert!(!std::path::Path::new("/sys/class/net/glr-wg0").exists());
    drop(_resume);
    let _: ConnectResult = c.call("connect", &p).await.unwrap();
    wait_state(&mut c, State::Connected, Duration::from_secs(5)).await;
    disconnect(&mut c).await;
}
