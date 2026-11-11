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
        let cfg = cfg(&h);
        let path = cfg.socket.clone();
        let shutdown = CancellationToken::new();
        let task = tokio::spawn(serve(cfg, h.handle.clone(), info(), shutdown.clone()));
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
