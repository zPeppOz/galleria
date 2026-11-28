#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use galleria_ipc::{client::*, *};
use galleriad::{
    config::DaemonConfig,
    server::{DaemonInfo, serve},
};
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};
use support::*;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
};
use tokio_util::sync::CancellationToken;
struct Server {
    h: Harness,
    path: PathBuf,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}
fn cfg(h: &Harness) -> DaemonConfig {
    DaemonConfig {
        socket: h.tmp.path().join("galleriad.sock"),
        group: nix::unistd::Group::from_gid(nix::unistd::getgid())
            .unwrap()
            .unwrap()
            .name,
        runtime_dir: h.tmp.path().to_path_buf(),
        hello_timeout: Duration::from_millis(200),
        max_clients: 16,
    }
}
fn info() -> DaemonInfo {
    DaemonInfo {
        backend_probe: None,
        version: "0.1.0".into(),
        backends: vec![],
    }
}
impl Server {
    async fn start() -> Self {
        let h = Harness::new(vec![online(), online()]);
        Self::with_h(h).await
    }
    async fn with_h(h: Harness) -> Self {
        Self::with_info(h, info()).await
    }
    async fn with_info(h: Harness, info: DaemonInfo) -> Self {
        let cfg = cfg(&h);
        let path = cfg.socket.clone();
        let shutdown = CancellationToken::new();
        let task = tokio::spawn(serve(cfg, h.handle.clone(), info, shutdown.clone()));
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                assert!(!task.is_finished(), "server stopped");
                if let Ok(mut probe) = Client::connect(&path).await
                    && probe.hello("readiness", "1").await.is_ok()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        Self {
            h,
            path,
            shutdown,
            task,
        }
    }
    async fn client(&self) -> Client {
        let mut c = Client::connect(&self.path).await.unwrap();
        c.hello("test", "1").await.unwrap();
        c
    }
    async fn stop(self) {
        self.shutdown.cancel();
        self.task.await.unwrap().unwrap();
        assert!(!self.path.exists());
    }
}
async fn raw(
    path: &std::path::Path,
    method: &str,
    params: Value,
) -> (BufReader<UnixStream>, Value) {
    let mut stream = UnixStream::connect(path).await.unwrap();
    stream
        .write_all(&encode_line(
            &json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}),
        ))
        .await
        .unwrap();
    let mut reader = BufReader::new(stream);
    let value = serde_json::from_slice(
        &read_frame(&mut reader, MAX_MESSAGE_BYTES)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    (reader, value)
}
#[tokio::test]
async fn hello_then_status_is_disconnected() {
    let s = Server::start().await;
    let mut c = Client::connect(&s.path).await.unwrap();
    assert_eq!(c.hello("test", "1").await.unwrap().protocol, 1);
    let status: Status = c.call("status", &json!({})).await.unwrap();
    assert_eq!(status.state, State::Disconnected);
    s.stop().await;
}
#[tokio::test]
async fn method_before_hello_is_hello_required() {
    let s = Server::start().await;
    let (_, v) = raw(&s.path, "status", json!({})).await;
    assert_eq!(v["error"]["data"]["code"], "hello_required");
    s.stop().await;
}
#[tokio::test]
async fn incompatible_protocol_closes_the_connection() {
    let s = Server::start().await;
    let (mut r, v) = raw(&s.path, "hello", json!({"protocol":2,"future":true})).await;
    assert_eq!(v["error"]["data"]["code"], "incompatible_protocol");
    assert!(
        read_frame(&mut r, MAX_MESSAGE_BYTES)
            .await
            .unwrap()
            .is_none()
    );
    s.stop().await;
}
#[tokio::test]
async fn oversized_line_is_message_too_large_then_close() {
    let s = Server::start().await;
    let mut stream = UnixStream::connect(&s.path).await.unwrap();
    stream
        .write_all(&vec![b'x'; MAX_MESSAGE_BYTES + 1])
        .await
        .unwrap();
    let mut r = BufReader::new(stream);
    let v: Value = serde_json::from_slice(
        &read_frame(&mut r, MAX_MESSAGE_BYTES)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(v["error"]["data"]["code"], "message_too_large");
    assert!(
        read_frame(&mut r, MAX_MESSAGE_BYTES)
            .await
            .unwrap()
            .is_none()
    );
    s.stop().await;
}
#[tokio::test]
async fn silent_client_is_dropped_without_blocking_others() {
    let s = Server::start().await;
    let mut r = BufReader::new(UnixStream::connect(&s.path).await.unwrap());
    let _other = tokio::time::timeout(Duration::from_millis(100), s.client())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_secs(2),
            read_frame(&mut r, MAX_MESSAGE_BYTES)
        )
        .await
        .unwrap()
        .unwrap()
        .is_none()
    );
    s.stop().await;
}
#[tokio::test]
async fn half_line_then_silence_is_dropped() {
    let s = Server::start().await;
    let mut stream = UnixStream::connect(&s.path).await.unwrap();
    stream.write_all(b"{\"jsonrpc\":").await.unwrap();
    let mut r = BufReader::new(stream);
    assert!(
        tokio::time::timeout(
            Duration::from_secs(2),
            read_frame(&mut r, MAX_MESSAGE_BYTES)
        )
        .await
        .unwrap()
        .unwrap()
        .is_none()
    );
    s.stop().await;
}
#[tokio::test]
async fn seventeenth_client_is_too_many_clients() {
    let s = Server::start().await;
    let mut clients = vec![];
    for _ in 0..16 {
        clients.push(s.client().await);
    }
    let mut r = BufReader::new(UnixStream::connect(&s.path).await.unwrap());
    let v: Value = serde_json::from_slice(
        &read_frame(&mut r, MAX_MESSAGE_BYTES)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(v["error"]["data"]["code"], "too_many_clients");
    assert!(
        read_frame(&mut r, MAX_MESSAGE_BYTES)
            .await
            .unwrap()
            .is_none()
    );
    s.stop().await;
}
#[tokio::test]
async fn connect_twice_is_already_active_then_replace_succeeds() {
    let s = Server::start().await;
    let mut c = s.client().await;
    let first: ConnectResult = c.call("connect", &params(false)).await.unwrap();
    let e = c
        .call::<_, ConnectResult>("connect", &params(false))
        .await
        .unwrap_err();
    let ClientError::Rpc(e) = e else {
        panic!("wrong error")
    };
    assert_eq!(e.data.unwrap()["code"], "already_active");
    let second: ConnectResult = c.call("connect", &params(true)).await.unwrap();
    assert_ne!(first.session, second.session);
    s.stop().await;
}
async fn connected_events(c: &mut Client) {
    let mut states = vec![];
    tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = c.next_event().await {
            if let Event::State(s) = event {
                states.push(s.state);
                if s.state == State::Connected {
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    assert!(states.contains(&State::Connecting));
    assert_eq!(states.last(), Some(&State::Connected));
}
#[tokio::test]
async fn state_events_reach_every_client() {
    let s = Server::start().await;
    let mut a = s.client().await;
    let mut b = s.client().await;
    let _: ConnectResult = a.call("connect", &params(false)).await.unwrap();
    connected_events(&mut a).await;
    connected_events(&mut b).await;
    s.stop().await;
}
#[tokio::test]
async fn client_dropping_mid_session_leaves_it_running() {
    let s = Server::start().await;
    let mut a = s.client().await;
    let _: ConnectResult = a.call("connect", &params(false)).await.unwrap();
    s.h.state(State::Connected).await;
    drop(a);
    let mut b = s.client().await;
    let status: Status = b.call("status", &json!({})).await.unwrap();
    assert_eq!(status.state, State::Connected);
    s.stop().await;
}
#[tokio::test]
async fn provide_input_with_wrong_request_is_input_not_expected() {
    let h = Harness::new(vec![vec![galleria_backend::fake::FakeStep::Emit(
        galleria_backend::fake::FakeEvent::InputRequired(InputRequest {
            request: 1,
            kind: InputKind::Otp,
            prompt: "OTP".into(),
            echo: false,
            expires_in_secs: 120,
        }),
    )]]);
    let s = Server::with_h(h).await;
    let mut c = s.client().await;
    let _: ConnectResult = c.call("connect", &params(false)).await.unwrap();
    s.h.state(State::NeedsInput).await;
    let e = c
        .call::<_, Value>(
            "provide_input",
            &json!({"session":1,"request":999,"values":{}}),
        )
        .await
        .unwrap_err();
    let ClientError::Rpc(e) = e else {
        panic!("wrong error")
    };
    assert_eq!(e.data.unwrap()["code"], "input_not_expected");
    s.stop().await;
}
#[tokio::test]
async fn socket_mode_is_0660() {
    let s = Server::start().await;
    assert_eq!(
        std::fs::metadata(&s.path).unwrap().permissions().mode() & 0o777,
        0o660
    );
    s.stop().await;
}
#[tokio::test]
async fn stale_socket_file_is_replaced() {
    let h = Harness::new(vec![]);
    let path = cfg(&h).socket;
    drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
    let s = Server::with_h(h).await;
    let _ = s.client().await;
    s.stop().await;
}
#[tokio::test]
async fn second_instance_refuses_to_start() {
    let s = Server::start().await;
    let second = serve(
        cfg(&s.h),
        s.h.handle.clone(),
        info(),
        CancellationToken::new(),
    )
    .await;
    assert!(second.is_err());
    let _ = s.client().await;
    s.stop().await;
}
#[tokio::test]
async fn sigterm_disconnects_before_exit() {
    let s = Server::start().await;
    let mut c = s.client().await;
    let _: ConnectResult = c.call("connect", &params(false)).await.unwrap();
    s.h.state(State::Connected).await;
    let net = s.h.net.clone();
    let live = s.h.live.clone();
    s.stop().await;
    let log = net.log.lock().unwrap();
    assert!(log.contains(&"teardown".into()));
    assert_eq!(live.load(std::sync::atomic::Ordering::SeqCst), 0);
}
#[tokio::test]
async fn orphan_session_dirs_are_purged_before_listening() {
    let h = Harness::new(vec![]);
    let p = h.tmp.path().join("s/vecchia");
    std::fs::create_dir_all(&p).unwrap();
    drop(std::os::unix::net::UnixListener::bind(p.join("mgmt.sock")).unwrap());
    let s = Server::with_h(h).await;
    assert_eq!(
        std::fs::read_dir(s.h.tmp.path().join("s")).unwrap().count(),
        0
    );
    let _ = s.client().await;
    s.stop().await;
}

#[tokio::test]
async fn partial_request_survives_interleaved_state_notifications() {
    let s = Server::start().await;
    let (mut reader, hello) = raw(
        &s.path,
        "hello",
        json!({"protocol":1,"client":{"name":"test","version":"1"}}),
    )
    .await;
    assert_eq!(hello["result"]["protocol"], 1);
    reader
        .get_mut()
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":42,")
        .await
        .unwrap();
    s.h.handle.connect(params(false), 1000).await.unwrap();
    s.h.state(State::Connected).await;
    let event: Value = serde_json::from_slice(
        &read_frame(&mut reader, MAX_MESSAGE_BYTES)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(event["method"], "event.state");
    reader
        .get_mut()
        .write_all(b"\"method\":\"status\",\"params\":{}}\n")
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let v: Value = serde_json::from_slice(
                &read_frame(&mut reader, MAX_MESSAGE_BYTES)
                    .await
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
            if v["id"] == 42 {
                assert_eq!(v["result"]["state"], "connected");
                break;
            }
        }
    })
    .await
    .unwrap();
    s.stop().await;
}

#[tokio::test]
async fn blocked_pre_hello_response_expires_and_releases_client_slot() {
    let s = Server::start().await;
    let mut peers = vec![];
    for _ in 0..16 {
        let mut peer = UnixStream::connect(&s.path).await.unwrap();
        let frame = encode_line(
            &json!({"jsonrpc":"2.0","id":"x".repeat(600_000),"method":"status","params":{}}),
        );
        peer.write_all(&frame).await.unwrap();
        peers.push(peer);
    }
    tokio::time::sleep(Duration::from_millis(400)).await;
    let mut client = Client::connect(&s.path).await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), client.hello("after deadline", "1"))
        .await
        .unwrap();
    assert!(
        result.is_ok(),
        "expired sockets still occupy slots: {result:?}"
    );
    drop(peers);
    s.stop().await;
}

async fn missing_group_server() -> Server {
    let h = Harness::new(vec![]);
    let mut cfg = cfg(&h);
    cfg.group = format!("galleria-no-group-{}", uuid::Uuid::new_v4());
    cfg.hello_timeout = Duration::from_secs(60);
    assert!(nix::unistd::Group::from_name(&cfg.group).unwrap().is_none());
    let path = cfg.socket.clone();
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(serve(cfg, h.handle.clone(), info(), shutdown.clone()));
    tokio::time::timeout(Duration::from_secs(2), async {
        while !path.exists() {
            assert!(!task.is_finished());
            tokio::task::yield_now().await;
        }
        // Round trip to the manager runs after serve has configured the bound socket.
        h.handle.status().await;
    })
    .await
    .unwrap();
    Server {
        h,
        path,
        shutdown,
        task,
    }
}
#[tokio::test]
async fn missing_group_socket_mode_is_0600() {
    let s = missing_group_server().await;
    assert_eq!(
        std::fs::metadata(&s.path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    s.stop().await;
}
#[tokio::test]
async fn unauthorized_client_is_closed_without_any_response() {
    if nix::unistd::getuid().is_root() {
        return;
    }
    let s = missing_group_server().await;
    // Connect as the owning, non-root user: DAC allows access, server policy must reject it.
    let peer = UnixStream::connect(&s.path).await.unwrap();
    let mut reader = BufReader::new(peer);
    // This deadline is shorter than hello_timeout: merely ignoring the client fails.
    assert!(
        tokio::time::timeout(
            Duration::from_millis(500),
            read_frame(&mut reader, MAX_MESSAGE_BYTES)
        )
        .await
        .unwrap()
        .unwrap()
        .is_none()
    );
    s.stop().await;
}

fn logging_script(message: String) -> Vec<galleria_backend::fake::FakeStep> {
    use galleria_backend::{LogLine, fake::*};
    let mut script = online();
    script.extend([
        FakeStep::Emit(FakeEvent::Log(LogLine {
            level: LogLevel::Info,
            message,
        })),
        FakeStep::Emit(FakeEvent::Stats(Stats {
            rx_bytes: 123,
            tx_bytes: 456,
            last_handshake: None,
        })),
        FakeStep::Emit(FakeEvent::Progress("log barrier".into())),
        // Progress is ignored in Connected; a log sentinel is observable by subscribers,
        // while a later NetUp republishes state for every client after Log and Stats.
        FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.9"))),
    ]);
    script
}
async fn through_last_state(client: &mut Client) -> Vec<Event> {
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut events = vec![];
        loop {
            let event = client.next_event().await.expect("connection closed");
            let done = matches!(&event, Event::State(s) if s.net.as_ref().is_some_and(|n| n.dns_servers[0].to_string() == "10.0.0.9"));
            events.push(event);
            if done { return events; }
        }
    }).await.unwrap()
}
#[tokio::test]
async fn stats_and_logs_only_reach_subscribers() {
    let s = Server::with_h(Harness::new(vec![logging_script("backend message".into())])).await;
    let mut clients = Vec::new();
    for (stats, logs) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut c = s.client().await;
        if stats || logs {
            let _: Value = c
                .call("subscribe", &json!({"stats":stats,"logs":logs}))
                .await
                .unwrap();
        } // The first client never subscribes: defaults must exclude stats and logs.
        clients.push((c, stats, logs));
    }
    let _: ConnectResult = clients[0].0.call("connect", &params(false)).await.unwrap();
    for (c, stats, logs) in &mut clients {
        let events = through_last_state(c).await;
        assert_eq!(
            events
                .iter()
                .any(|e| matches!(e,Event::Stats(v) if v.rx_bytes == 123)),
            *stats
        );
        assert_eq!(
            events
                .iter()
                .any(|e| matches!(e,Event::Log(v) if v.message == "backend message")),
            *logs
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e,Event::State(v) if v.state == State::Connected))
        );
    }
    s.stop().await;
}
#[tokio::test]
async fn secrets_from_connect_never_reach_any_client() {
    use galleria_backend::{Backend, fake::FakeBackend};
    use galleriad::{
        machine::RetryPolicy,
        manager::{BackendFactory, spawn_manager},
    };
    use std::sync::{Arc, atomic::AtomicUsize};
    let mut h = Harness::new(vec![]);
    let factory: BackendFactory = Arc::new(|backend, _| {
        let password = match backend {
            BackendParams::Wireguard(p) => p.private_key.expose(),
            BackendParams::Openvpn(p) => p.password.as_ref().unwrap().expose(),
        };
        Ok(Box::new(FakeBackend::new(
            logging_script(format!("pw={password} ok")),
            Arc::new(AtomicUsize::new(0)),
        )) as Box<dyn Backend>)
    });
    h.handle = spawn_manager(
        factory,
        h.net.clone(),
        RetryPolicy::default(),
        h.tmp.path().to_path_buf(),
    );
    let s = Server::with_h(h).await;
    let mut clients = vec![s.client().await, s.client().await];
    let _: Value = clients[1]
        .call("subscribe", &json!({"stats":true,"logs":true}))
        .await
        .unwrap();
    let mut p = params(false);
    if let BackendParams::Wireguard(wg) = &mut p.backend {
        wg.private_key = Secret::new("test-only-password");
    }
    let _: ConnectResult = clients[0].call("connect", &p).await.unwrap();
    for c in &mut clients {
        for event in through_last_state(c).await {
            let frame = String::from_utf8(notification(&event)).unwrap();
            assert!(!frame.contains("test-only-password"), "unfiltered event");
        }
        let logs: LogsResult = c
            .call("logs", &json!({"after":null,"limit":1000}))
            .await
            .unwrap();
        assert!(logs.entries.iter().any(|e| e.message == "pw=[redatto] ok"));
        assert!(
            !serde_json::to_string(&logs)
                .unwrap()
                .contains("test-only-password")
        );
    }
    s.stop().await;
}
#[tokio::test]
async fn buffer_is_cleared_on_new_session() {
    let s = Server::with_h(Harness::new(vec![
        logging_script("first-session-only".into()),
        online(),
    ]))
    .await;
    let mut c = s.client().await;
    let _: ConnectResult = c.call("connect", &params(false)).await.unwrap();
    through_last_state(&mut c).await;
    let before: LogsResult = c
        .call("logs", &json!({"after":null,"limit":1000}))
        .await
        .unwrap();
    assert!(
        before
            .entries
            .iter()
            .any(|e| e.message == "first-session-only")
    );
    let _: ConnectResult = c.call("connect", &params(true)).await.unwrap();
    let after: LogsResult = c
        .call("logs", &json!({"after":null,"limit":1000}))
        .await
        .unwrap();
    assert!(
        !after
            .entries
            .iter()
            .any(|e| e.message == "first-session-only")
    );
    assert!(after.last > before.last);
    s.stop().await;
}
#[tokio::test]
async fn audit_line_records_uid_and_method() {
    let s = Server::start().await;
    let mut c = s.client().await;
    let _: ConnectResult = c.call("connect", &params(false)).await.unwrap();
    let logs: LogsResult = c
        .call("logs", &json!({"after":null,"limit":1000}))
        .await
        .unwrap();
    let uid = nix::unistd::getuid().as_raw();
    assert!(logs.entries.iter().any(|e| e.source == LogSource::Daemon
        && e.message.contains(&format!("uid {uid}"))
        && e.message.contains("connect")));
    s.stop().await;
}

#[tokio::test]
async fn credentials_from_provide_input_are_filtered_before_backend_logs_and_status() {
    use galleria_backend::{LogLine, fake::*};
    let h = Harness::new(vec![vec![
        FakeStep::Emit(FakeEvent::InputRequired(InputRequest {
            request: 5,
            kind: InputKind::Otp,
            prompt: "OTP".into(),
            echo: false,
            expires_in_secs: 120,
        })),
        FakeStep::WaitCommand,
        FakeStep::Emit(FakeEvent::Log(LogLine {
            level: LogLevel::Info,
            message: "new-input-value".into(),
        })),
        FakeStep::Emit(FakeEvent::Progress("new-input-value".into())),
        FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.9"))),
        up(),
    ]]);
    let s = Server::with_h(h).await;
    let mut c = s.client().await;
    let _: Value = c
        .call("subscribe", &json!({"stats":false,"logs":true}))
        .await
        .unwrap();
    let _: ConnectResult = c.call("connect", &params(false)).await.unwrap();
    s.h.state(State::NeedsInput).await;
    let _: Value = c
        .call(
            "provide_input",
            &json!({"session":1,"request":5,"values":{"otp":"new-input-value"}}),
        )
        .await
        .unwrap();
    for event in through_last_state(&mut c).await {
        assert!(
            !String::from_utf8(notification(&event))
                .unwrap()
                .contains("new-input-value")
        );
    }
    let logs: LogsResult = c
        .call("logs", &json!({"after":null,"limit":1000}))
        .await
        .unwrap();
    assert!(logs.entries.iter().any(|e| e.message == "[redatto]"));
    let status: Status = c.call("status", &json!({})).await.unwrap();
    assert_eq!(status.detail.as_deref(), Some("[redatto]"));
    s.stop().await;
}

#[tokio::test(flavor = "current_thread")]
async fn daemon_recovers_before_listening() {
    use galleria_net::{Cmd, CommandRunner, NetError, NetManager};
    use galleriad::{machine::RetryPolicy, manager::*};
    use std::sync::{Arc, Mutex};
    struct Runner {
        socket: PathBuf,
        calls: Arc<Mutex<Vec<String>>>,
        runtime_thread: std::thread::ThreadId,
    }
    impl CommandRunner for Runner {
        fn run(&self, cmd: &Cmd) -> Result<(), NetError> {
            assert!(
                !self.socket.exists(),
                "network recovery happened after bind"
            );
            assert_ne!(
                std::thread::current().id(),
                self.runtime_thread,
                "network I/O blocks the async runtime"
            );
            self.calls.lock().unwrap().push(cmd.to_string());
            Ok(())
        }
    }
    let h = Harness::new(vec![]);
    let cfg = cfg(&h);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let net = NetManager::new(
        Runner {
            socket: cfg.socket.clone(),
            calls: calls.clone(),
            runtime_thread: std::thread::current().id(),
        },
        h.tmp.path().join("net-journal.json"),
    );
    let manager = spawn_manager(
        Arc::new(|_, _| Err(ErrorCode::BackendUnavailable.to_rpc("test"))),
        Arc::new(net),
        RetryPolicy::default(),
        cfg.runtime_dir.clone(),
    );
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(serve(cfg.clone(), manager, info(), shutdown.clone()));
    tokio::time::timeout(Duration::from_secs(2), async {
        while !cfg.socket.exists() {
            assert!(!task.is_finished());
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(calls.lock().unwrap().len(), 6);
    shutdown.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn hello_refreshes_backend_availability() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let available = Arc::new(AtomicBool::new(false));
    let state = available.clone();
    let mut metadata = info();
    metadata.backend_probe = Some(Arc::new(move || {
        vec![BackendInfo {
            protocol: Protocol::Wireguard,
            available: state.load(Ordering::SeqCst),
            version: None,
            unavailable_reason: None,
            capabilities: Capabilities {
                handshake: true,
                interactive_auth: false,
                server_pushed_config: false,
            },
        }]
    }));
    let s = Server::with_info(Harness::new(vec![]), metadata).await;
    let mut c = Client::connect(&s.path).await.unwrap();
    assert!(!c.hello("test", "1").await.unwrap().backends[0].available);
    available.store(true, Ordering::SeqCst);
    let mut c = Client::connect(&s.path).await.unwrap();
    assert!(c.hello("test", "1").await.unwrap().backends[0].available);
    s.stop().await;
}
