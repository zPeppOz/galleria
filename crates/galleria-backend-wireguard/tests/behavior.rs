#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_backend::{net::*, *};
use galleria_backend_wireguard::*;
use galleria_ipc::*;
use rstest::rstest;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
fn params() -> WireGuardParams {
    serde_json::from_value(serde_json::json!({"private_key":"AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=","addresses":["10.99.0.2/24"],"dns":["10.99.0.1"],"search_domains":[],"mtu":1420,"peers":[{"public_key":"AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=","preshared_key":null,"endpoint":"127.0.0.1:51820","allowed_ips":["10.99.0.0/24","198.51.100.1/32"],"persistent_keepalive":null}]})).unwrap()
}
fn options() -> ConnectOptions {
    ConnectOptions {
        ipv6_block: true,
        ignore_pushed_routes: false,
        reconnect: ReconnectOptions { enabled: false },
    }
}
#[rstest]
#[case("private_key")]
#[case("addresses")]
#[case("peers")]
#[case("mtu")]
#[case("peers.0.endpoint")]
#[case("peers.0.public_key")]
#[case("peers.0.preshared_key")]
#[case("peers.0.allowed_ips")]
#[case("peers.0.persistent_keepalive")]
#[case("dns")]
#[case("search_domains")]
fn validate_rejects_invalid_fields(#[case] field: &str) {
    let mut p = params();
    match field {
        "private_key" => {
            p.private_key = Secret::new("AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQ==")
        }
        "addresses" => p.addresses = vec!["10.0.0.1/33".into()],
        "peers" => p.peers.clear(),
        "mtu" => p.mtu = Some(100),
        "peers.0.endpoint" => p.peers[0].endpoint = Some("missing-port".into()),
        "peers.0.public_key" => p.peers[0].public_key = "bad".into(),
        "peers.0.preshared_key" => p.peers[0].preshared_key = Some(Secret::new("bad")),
        "peers.0.allowed_ips" => p.peers[0].allowed_ips = vec!["not-a-route".into()],
        "peers.0.persistent_keepalive" => p.peers[0].persistent_keepalive = Some(65536),
        "dns" => p.dns = vec!["not-an-ip".into()],
        "search_domains" => p.search_domains = vec!["--evil".into()],
        _ => unreachable!(),
    }
    let e = validate(&p).err().expect("invalid configuration accepted");
    assert_eq!(e.data.as_ref().unwrap()["code"], "invalid_config");
    assert_eq!(e.data.unwrap()["field"], field);
}
#[test]
fn net_config_split_tunnel() {
    let cfg = net_config(&validate(&params()).unwrap(), true);
    assert_eq!(
        cfg.routes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["10.99.0.0/24", "198.51.100.1/32"]
    );
    assert_eq!(cfg.install_routes, Some(51820));
    assert_eq!(cfg.ipv6_block, Ipv6Block::None);
}
#[test]
fn net_config_full_tunnel_requests_unreachable_block() {
    let mut p = params();
    p.peers[0].allowed_ips = vec!["0.0.0.0/0".into()];
    assert_eq!(
        net_config(&validate(&p).unwrap(), true).ipv6_block,
        Ipv6Block::Unreachable
    );
}
#[test]
fn net_config_full_tunnel_with_ipv6_default_requests_no_block() {
    let mut p = params();
    p.peers[0].allowed_ips = vec!["0.0.0.0/0".into(), "::/0".into()];
    assert_eq!(
        net_config(&validate(&p).unwrap(), true).ipv6_block,
        Ipv6Block::None
    );
}
#[test]
fn ipv6_block_option_disables_block() {
    let mut p = params();
    p.peers[0].allowed_ips = vec!["0.0.0.0/0".into()];
    assert_eq!(
        net_config(&validate(&p).unwrap(), false).ipv6_block,
        Ipv6Block::None
    );
}
#[derive(Default)]
struct Observed {
    calls: Vec<&'static str>,
    endpoint: Option<std::net::SocketAddr>,
    fwmark: Option<u32>,
    handshake: Option<SystemTime>,
}
struct Device(Arc<Mutex<Observed>>);
impl WgDevice for Device {
    fn create(&mut self) -> Result<(), WgError> {
        self.0.lock().unwrap().calls.push("create");
        Ok(())
    }
    fn configure(&mut self, c: &DeviceConfig) -> Result<(), WgError> {
        let mut o = self.0.lock().unwrap();
        o.calls.push("configure");
        o.endpoint = c.peers[0].endpoint;
        o.fwmark = c.fwmark;
        Ok(())
    }
    fn read(&self) -> Result<DeviceStats, WgError> {
        let mut o = self.0.lock().unwrap();
        o.calls.push("read");
        Ok(DeviceStats {
            rx_bytes: 10,
            tx_bytes: 20,
            last_handshake: o.handshake,
        })
    }
    fn remove(&mut self) -> Result<(), WgError> {
        self.0.lock().unwrap().calls.push("remove");
        Ok(())
    }
}
fn spawn(
    p: WireGuardParams,
) -> (
    SessionHandle,
    mpsc::Receiver<BackendEvent>,
    Arc<Mutex<Observed>>,
) {
    let state = Arc::new(Mutex::new(Observed::default()));
    let (events, rx) = mpsc::channel(128);
    let backend = WireGuardBackend::new(
        validate(&p).unwrap(),
        &options(),
        Box::new(Device(state.clone())),
    );
    (
        Box::new(backend).spawn(SessionContext {
            session: 1,
            session_dir: std::path::PathBuf::new(),
            events,
        }),
        rx,
        state,
    )
}
async fn event(rx: &mut mpsc::Receiver<BackendEvent>) -> BackendEvent {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap()
}
async fn accept(rx: &mut mpsc::Receiver<BackendEvent>) {
    match event(rx).await {
        BackendEvent::NetUp { reply, .. } => reply.send(Ok(())).unwrap(),
        _ => panic!("expected NetUp"),
    };
    assert!(matches!(event(rx).await, BackendEvent::Up(_)));
}
async fn stop(h: SessionHandle, rx: &mut mpsc::Receiver<BackendEvent>) {
    h.commands.send(BackendCommand::Stop).await.unwrap();
    while !matches!(event(rx).await, BackendEvent::Exited(ExitReason::Stopped)) {}
    h.task.await.unwrap();
}
#[tokio::test]
async fn session_calls_device_in_order() {
    let (h, mut rx, o) = spawn(params());
    accept(&mut rx).await;
    stop(h, &mut rx).await;
    assert_eq!(o.lock().unwrap().calls, ["create", "configure", "remove"]);
}
#[tokio::test]
async fn net_refusal_removes_device_and_exits_fatal() {
    let (h, mut rx, o) = spawn(params());
    match event(&mut rx).await {
        BackendEvent::NetUp { reply, .. } => reply.send(Err("refused".into())).unwrap(),
        _ => panic!(),
    };
    assert!(matches!(
        event(&mut rx).await,
        BackendEvent::Exited(ExitReason::Fatal(SessionError {
            code: SessionErrorCode::NetSetupFailed,
            ..
        }))
    ));
    h.task.await.unwrap();
    assert_eq!(o.lock().unwrap().calls, ["create", "configure", "remove"]);
}
#[tokio::test]
async fn unresolvable_endpoint_exits_transport() {
    let mut p = params();
    p.peers[0].endpoint = Some("nonexistent.invalid:51820".into());
    let (h, mut rx, o) = spawn(p);
    assert!(matches!(
        event(&mut rx).await,
        BackendEvent::Exited(ExitReason::Transport(_))
    ));
    h.task.await.unwrap();
    assert!(o.lock().unwrap().calls.is_empty());
}
#[tokio::test]
async fn ipv6_only_endpoint_is_accepted() {
    let mut p = params();
    p.peers[0].endpoint = Some("[2001:db8::1]:51820".into());
    let (h, mut rx, o) = spawn(p);
    accept(&mut rx).await;
    assert_eq!(
        o.lock().unwrap().endpoint,
        Some("[2001:db8::1]:51820".parse().unwrap())
    );
    stop(h, &mut rx).await;
}
#[tokio::test]
async fn fwmark_is_set_only_for_full_tunnel() {
    for (routes, mark) in [
        (vec!["10.0.0.0/8"], None),
        (vec!["0.0.0.0/0"], Some(51820)),
        (vec!["::/0"], Some(51820)),
    ] {
        let mut p = params();
        p.peers[0].allowed_ips = routes.into_iter().map(String::from).collect();
        let (h, mut rx, o) = spawn(p);
        accept(&mut rx).await;
        assert_eq!(o.lock().unwrap().fwmark, mark);
        stop(h, &mut rx).await;
    }
}
#[tokio::test]
async fn up_does_not_wait_for_handshake() {
    let (h, mut rx, o) = spawn(params());
    accept(&mut rx).await;
    assert!(o.lock().unwrap().handshake.is_none());
    stop(h, &mut rx).await;
}
// A blocking worker owns the device; paused Tokio time must not advance before NetUp/Up barriers.
#[tokio::test(start_paused = true)]
async fn stats_are_emitted_every_second() {
    let (h, mut rx, _) = spawn(params());
    accept(&mut rx).await;
    for _ in 0..3 {
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(matches!(
            event(&mut rx).await,
            BackendEvent::Stats(Stats {
                rx_bytes: 10,
                tx_bytes: 20,
                ..
            })
        ));
    }
    stop(h, &mut rx).await;
}
async fn warning_case(keepalive: bool, handshake: bool) -> usize {
    let mut p = params();
    if keepalive {
        p.peers[0].persistent_keepalive = Some(25);
    }
    let (h, mut rx, o) = spawn(p);
    o.lock().unwrap().handshake = handshake.then(SystemTime::now);
    accept(&mut rx).await;
    let mut warnings = 0;
    for _ in 0..12 {
        tokio::time::advance(Duration::from_secs(1)).await;
        loop {
            match event(&mut rx).await {
                BackendEvent::Log(l) => {
                    assert_eq!(l.level, LogLevel::Warn);
                    assert_eq!(l.message, "no handshake yet");
                    warnings += 1;
                }
                BackendEvent::Stats(_) => break,
                _ => panic!("unexpected state/exit event"),
            }
        }
    }
    h.commands.send(BackendCommand::Stop).await.unwrap();
    h.task.await.unwrap();
    while let Some(e) = rx.recv().await {
        if let BackendEvent::Log(_) = e {
            warnings += 1;
        }
    }
    warnings
}
#[tokio::test(start_paused = true)]
async fn warns_once_after_10s_without_handshake_and_without_keepalive() {
    assert_eq!(warning_case(false, false).await, 1);
}
#[tokio::test(start_paused = true)]
async fn no_handshake_warning_when_keepalive_is_set() {
    assert_eq!(warning_case(true, false).await, 0);
}
#[tokio::test(start_paused = true)]
async fn no_handshake_warning_when_handshake_happened() {
    assert_eq!(warning_case(false, true).await, 0);
}
