#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
use galleria_ipc::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[tokio::test]
#[ignore]
async fn hello_reports_wireguard_available() {
    let mut c = client().await;
    let h = c.hello("test", "1").await.unwrap();
    assert!(
        h.backends
            .iter()
            .any(|b| b.protocol == Protocol::Wireguard && b.available)
    );
}
#[tokio::test]
#[ignore]
async fn wg_split_connects_and_pings_server() {
    let mut c = client().await;
    connect_wg(&mut c, "wg-split.conf").await;
    checked("ping -c 3 -W 2 10.99.0.1");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn wg_split_routes_target_through_tunnel() {
    let mut c = client().await;
    connect_wg(&mut c, "wg-split.conf").await;
    assert!(checked("ip route get 198.51.100.1").contains("dev glr-wg0"));
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
async fn wg_stats_grow_and_handshake_is_recent() {
    let mut c = client().await;
    connect_wg(&mut c, "wg-split.conf").await;
    let _: serde_json::Value = c
        .call(
            "subscribe",
            &SubscribeParams {
                stats: true,
                logs: false,
            },
        )
        .await
        .unwrap();
    checked("ping -c 2 -W 2 10.99.0.1");
    let first = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(Event::Stats(s)) = c.next_event().await {
                break s;
            }
        }
    })
    .await
    .unwrap();
    checked("ping -c 4 -i .3 -W 2 10.99.0.1");
    let last = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(Event::Stats(s)) = c.next_event().await
                && s.rx_bytes > first.rx_bytes
            {
                break s;
            }
        }
    })
    .await
    .unwrap();
    let ts = last.last_handshake.unwrap().parse::<u64>().unwrap();
    assert!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .abs_diff(ts)
            < 30
    );
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn wg_dns_is_applied_before_connected() {
    let mut c = client().await;
    disconnect(&mut c).await;
    let _: serde_json::Value = c
        .call("subscribe", &SubscribeParams::default())
        .await
        .unwrap();
    let _: ConnectResult = c
        .call("connect", &wg_params("wg-split.conf"))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(Event::State(s)) = c.next_event().await
                && s.state == State::Connected
            {
                assert!(checked("resolvectl dns glr-wg0").contains("10.99.0.1"));
                break;
            }
        }
    })
    .await
    .unwrap();
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn wg_disconnect_leaves_no_residue() {
    let mut c = client().await;
    disconnect(&mut c).await;
    let before = residue();
    connect_wg(&mut c, "wg-split.conf").await;
    disconnect(&mut c).await;
    assert_eq!(residue(), before);
}
#[tokio::test]
#[ignore]
async fn wg_replace_switches_session() {
    let mut c = client().await;
    let s = connect_wg(&mut c, "wg-split.conf").await;
    let e = c
        .call::<_, ConnectResult>("connect", &wg_params("wg-split.conf"))
        .await
        .unwrap_err();
    assert!(
        matches!(e,client::ClientError::Rpc(e) if e.data.as_ref().unwrap()["code"]=="already_active")
    );
    let mut p = wg_params("wg-split.conf");
    p.replace = true;
    let second: ConnectResult = c.call("connect", &p).await.unwrap();
    assert_ne!(Some(second.session), s.session);
    wait_state(&mut c, State::Connected, Duration::from_secs(5)).await;
    assert_eq!(checked("ip -o link show glr-wg0").lines().count(), 1);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn invalid_key_is_invalid_config() {
    let mut c = client().await;
    disconnect(&mut c).await;
    let mut p = wg_params("wg-split.conf");
    if let BackendParams::Wireguard(w) = &mut p.backend {
        w.private_key = Secret::new("invalid");
    }
    let e = c.call::<_, ConnectResult>("connect", &p).await.unwrap_err();
    assert!(
        matches!(e,client::ClientError::Rpc(e) if e.data.as_ref().unwrap()["code"]=="invalid_config" && e.data.as_ref().unwrap()["field"]=="private_key")
    );
}
