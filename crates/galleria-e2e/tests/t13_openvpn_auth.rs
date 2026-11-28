#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
use galleria_ipc::*;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

fn request(profile: &str) -> ConnectParams {
    ConnectParams {
        profile: ProfileRef {
            id: format!("auth-{profile}"),
            name: "Auth laboratory".into(),
        },
        replace: false,
        backend: BackendParams::Openvpn(ovpn_params(profile)),
        options: ConnectOptions {
            ipv6_block: true,
            ignore_pushed_routes: false,
            reconnect: ReconnectOptions { enabled: true },
        },
    }
}
fn ovpn(p: &mut ConnectParams) -> &mut OpenVpnParams {
    let BackendParams::Openvpn(p) = &mut p.backend else {
        panic!()
    };
    p
}
async fn clean() -> client::Client {
    let mut c = client().await;
    disconnect(&mut c).await;
    let _: serde_json::Value = c
        .call(
            "subscribe",
            &SubscribeParams {
                stats: false,
                logs: true,
            },
        )
        .await
        .unwrap();
    c
}
async fn state(c: &mut client::Client, wanted: State, secs: u64) -> Status {
    tokio::time::timeout(Duration::from_secs(secs), async {
        loop {
            if let Some(Event::State(s)) = c.next_event().await {
                assert!(
                    s.state != State::Error || wanted == State::Error,
                    "{:?}",
                    s.error
                );
                if s.state == wanted {
                    return s;
                }
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no {wanted:?} within {secs}s"))
}
async fn start(c: &mut client::Client, p: &ConnectParams) {
    let _: ConnectResult = c.call("connect", p).await.unwrap();
}
async fn supply(c: &mut client::Client, s: &Status, values: InputValues) {
    let _: serde_json::Value = c
        .call(
            "provide_input",
            &ProvideInputParams {
                session: s.session.unwrap(),
                request: s.input_request.as_ref().unwrap().request,
                values,
            },
        )
        .await
        .unwrap();
}
fn otp() -> Secret {
    Secret::new(checked("oathtool --totp -b JBSWY3DPEHPK3PXP"))
}
fn bob() -> ConnectParams {
    let mut p = request("otp");
    ovpn(&mut p).username = Some("bob".into());
    ovpn(&mut p).password = Some(Secret::new("bob-secret"));
    p
}
struct Blackhole;
impl Blackhole {
    fn add() -> Self {
        // A separate policy table survives OpenVPN 2.6/2.7 endpoint-route churn.
        checked("ip route add blackhole default table 19999");
        checked("ip rule add priority 100 to 172.31.0.10/32 lookup 19999");
        Self
    }
}
impl Drop for Blackhole {
    fn drop(&mut self) {
        let _ = sh("ip rule del priority 100 to 172.31.0.10/32 lookup 19999");
        let _ = sh("ip route del blackhole default table 19999");
    }
}
fn no_network() {
    assert!(
        !sh("resolvectl dns glr-ovpn0")
            .stdout
            .windows(8)
            .any(|w| w == b"10.8.0.1")
    );
    assert!(!checked("ip -6 route").contains("unreachable"));
}
fn network() {
    assert!(checked("resolvectl dns glr-ovpn0").contains("10.8.0.1"));
    assert!(checked("ip -6 route").contains("unreachable"));
}
#[tokio::test]
#[ignore]
async fn missing_password_asks_then_connects() {
    let mut c = clean().await;
    let mut p = request("base");
    ovpn(&mut p).password = None;
    start(&mut c, &p).await;
    let s = state(&mut c, State::NeedsInput, 10).await;
    assert_eq!(
        s.input_request.as_ref().unwrap().kind,
        InputKind::Credentials
    );
    supply(
        &mut c,
        &s,
        InputValues {
            password: Some(Secret::new("alice-secret")),
            ..Default::default()
        },
    )
    .await;
    state(&mut c, State::Connected, 15).await;
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn otp_static_challenge_prompts_and_connects() {
    let mut c = clean().await;
    start(&mut c, &bob()).await;
    let s = state(&mut c, State::NeedsInput, 10).await;
    let r = s.input_request.as_ref().unwrap();
    assert_eq!(r.kind, InputKind::Otp);
    assert_eq!(r.prompt, "Enter OTP");
    supply(
        &mut c,
        &s,
        InputValues {
            otp: Some(otp()),
            ..Default::default()
        },
    )
    .await;
    state(&mut c, State::Connected, 15).await;
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn wrong_password_is_auth_failed_without_retry() {
    let mut c = clean().await;
    let mut p = request("base");
    ovpn(&mut p).password = Some(Secret::new("wrong"));
    start(&mut c, &p).await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(Event::State(s)) = c.next_event().await {
                assert!(s.attempt <= 1);
                if s.state == State::Error {
                    assert_eq!(s.error.unwrap().code, SessionErrorCode::AuthFailed);
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    assert!(!sh("pgrep -x openvpn").status.success());
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn wrong_otp_is_auth_failed() {
    let mut c = clean().await;
    let mut p = bob();
    ovpn(&mut p).otp = Some(Secret::new("invalid-otp"));
    start(&mut c, &p).await;
    let s = state(&mut c, State::Error, 10).await;
    assert_eq!(s.error.unwrap().code, SessionErrorCode::AuthFailed);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn encrypted_key_asks_for_passphrase() {
    let mut c = clean().await;
    start(&mut c, &request("enckey")).await;
    let s = state(&mut c, State::NeedsInput, 10).await;
    assert_eq!(
        s.input_request.as_ref().unwrap().kind,
        InputKind::KeyPassphrase
    );
    supply(
        &mut c,
        &s,
        InputValues {
            key_passphrase: Some(Secret::new("key-passphrase")),
            ..Default::default()
        },
    )
    .await;
    state(&mut c, State::Connected, 15).await;
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn unreachable_server_gives_up_after_four_attempts() {
    let mut c = clean().await;
    let start_time = Instant::now();
    start(&mut c, &request("unreachable")).await;
    let attempts = tokio::time::timeout(Duration::from_secs(45), async {
        let mut attempts = BTreeSet::new();
        loop {
            if let Some(Event::State(s)) = c.next_event().await {
                if s.attempt > 0 {
                    attempts.insert(s.attempt);
                }
                if s.state == State::Error {
                    assert_eq!(s.error.unwrap().code, SessionErrorCode::ServerUnreachable);
                    break attempts;
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(attempts, [1, 2, 3, 4].into_iter().collect());
    assert!((20..=45).contains(&start_time.elapsed().as_secs()));
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn internal_restart_returns_to_reconnecting_then_connected() {
    let mut c = clean().await;
    start(&mut c, &request("base")).await;
    let first = state(&mut c, State::Connected, 15).await;
    let pid = checked("pgrep -xo openvpn");
    checked("pkill -USR1 -x openvpn");
    let next = state(&mut c, State::Reconnecting, 10).await;
    assert_eq!(next.session, first.session);
    let connected = state(&mut c, State::Connected, 30).await;
    assert_eq!(connected.session, first.session);
    assert_eq!(checked("pgrep -xo openvpn"), pid);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn internal_restart_removes_and_reapplies_dns_and_ipv6_block() {
    let mut c = clean().await;
    let before = residue();
    start(&mut c, &request("base")).await;
    state(&mut c, State::Connected, 15).await;
    network();
    let blackhole = Blackhole::add();
    checked("pkill -USR1 -x openvpn");
    state(&mut c, State::Reconnecting, 10).await;
    no_network();
    drop(blackhole);
    state(&mut c, State::Connected, 30).await;
    network();
    disconnect(&mut c).await;
    assert_eq!(residue(), before);
}
#[tokio::test]
#[ignore]
async fn blackholed_server_triggers_reconnect_and_recovery() {
    let mut c = clean().await;
    start(&mut c, &request("base")).await;
    state(&mut c, State::Connected, 15).await;
    let pid = checked("pgrep -xo openvpn");
    let blackhole = Blackhole::add();
    state(&mut c, State::Reconnecting, 20).await;
    no_network();
    tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            let s: Status = c.call("status", &serde_json::json!({})).await.unwrap();
            let process = sh("pgrep -xo openvpn");
            if s.attempt > 1
                && process.status.success()
                && String::from_utf8_lossy(&process.stdout).trim() != pid
            {
                break;
            }
            assert_ne!(s.state, State::Error, "{:?}", s.error);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    drop(blackhole);
    wait_state(&mut c, State::Connected, Duration::from_secs(60)).await;
    network();
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn reconnect_of_otp_profile_asks_for_a_new_otp() {
    let mut c = clean().await;
    let mut p = bob();
    ovpn(&mut p).otp = Some(otp());
    start(&mut c, &p).await;
    state(&mut c, State::Connected, 15).await;
    let blackhole = Blackhole::add();
    state(&mut c, State::Reconnecting, 20).await;
    let s = state(&mut c, State::NeedsInput, 45).await;
    assert_eq!(s.input_request.as_ref().unwrap().kind, InputKind::Otp);
    drop(blackhole);
    supply(
        &mut c,
        &s,
        InputValues {
            otp: Some(otp()),
            ..Default::default()
        },
    )
    .await;
    state(&mut c, State::Connected, 60).await;
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn auth_nocache_survives_renegotiation() {
    let mut c = clean().await;
    let mut p = request("base");
    ovpn(&mut p).config.push_str("auth-nocache\nreneg-sec 10\n");
    start(&mut c, &p).await;
    state(&mut c, State::Connected, 15).await;
    let until = tokio::time::Instant::now() + Duration::from_secs(25);
    let mut auth_requests = 0;
    loop {
        match tokio::time::timeout_at(until, c.next_event()).await {
            Err(_) => break,
            Ok(Some(Event::State(s))) => {
                assert_eq!(s.state, State::Connected);
            }
            Ok(Some(Event::Log(l))) if l.message.contains("soft reset") => {
                auth_requests += 1;
            }
            Ok(Some(_)) => {}
            Ok(None) => panic!("IPC closed"),
        }
    }
    let s: Status = c.call("status", &serde_json::json!({})).await.unwrap();
    assert_eq!(s.state, State::Connected);
    assert!(auth_requests > 0, "renegotiation must have happened");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn disconnect_during_needs_input_cleans_up() {
    let mut c = clean().await;
    let before = residue();
    let mut p = request("base");
    ovpn(&mut p).password = None;
    start(&mut c, &p).await;
    state(&mut c, State::NeedsInput, 10).await;
    disconnect(&mut c).await;
    assert!(!sh("pgrep -x openvpn").status.success());
    assert_eq!(residue(), before);
}
#[tokio::test]
#[ignore]
async fn known_secrets_never_appear_in_logs_or_journal() {
    let mut c = clean().await;
    let mut p = bob();
    let code = otp();
    ovpn(&mut p).otp = Some(code.clone());
    start(&mut c, &p).await;
    state(&mut c, State::Connected, 15).await;
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
    let journal = checked("journalctl -u galleriad --no-pager -o cat");
    let key = ovpn(&mut p).secret_blocks["key"]
        .expose()
        .lines()
        .find(|l| l.len() > 40 && !l.starts_with('-'))
        .unwrap();
    for secret in ["bob-secret", "alice-secret", code.expose(), key] {
        assert!(!journal.contains(secret), "journal contains known secret");
        assert!(
            logs.entries.iter().all(|l| !l.message.contains(secret)),
            "IPC contains known secret"
        );
    }
    disconnect(&mut c).await;
}
