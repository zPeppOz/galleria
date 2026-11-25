//! Level-2 tests of the event supervisor: a real daemon with a fake backend, plus scripted servers.
//! Every wait is "until X shows up, at most N seconds"; the only fixed sleep is the slow sink of one test.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;

use galleria_backend::{LogLine, fake::*};
use galleria_gui::{
    link::{DaemonView, LinkState},
    supervisor::{EventSink, RelinkPolicy, SupervisorHandle, spawn},
};
use galleria_ipc::{
    HelloResult, LogEntry, LogLevel, MAX_MESSAGE_BYTES, State, Stats, Status, encode_line,
    read_frame,
};
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
use support::*;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

const WAIT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
enum Rec {
    Link(Box<DaemonView>),
    State(Box<Status>),
    Stats(Stats),
    Log(LogEntry),
}

/// An `EventSink` that queues everything it is given; `log_delay` makes every `log` call slow.
struct Recorder {
    tx: UnboundedSender<Rec>,
    log_delay: Duration,
}
impl EventSink for Recorder {
    fn link(&self, view: &DaemonView) {
        let _ = self.tx.send(Rec::Link(Box::new(view.clone())));
    }
    fn state(&self, status: &Status) {
        let _ = self.tx.send(Rec::State(Box::new(status.clone())));
    }
    fn stats(&self, stats: &Stats) {
        let _ = self.tx.send(Rec::Stats(stats.clone()));
    }
    fn log(&self, entry: &LogEntry) {
        std::thread::sleep(self.log_delay);
        let _ = self.tx.send(Rec::Log(entry.clone()));
    }
}

/// What the recorder has received so far, plus waiting helpers.
struct Seen {
    rx: UnboundedReceiver<Rec>,
    all: Vec<Rec>,
}
impl Seen {
    /// Waits until a record matching `pred` arrives (at most `limit`) and returns it; everything is kept in `all`.
    async fn until_within(&mut self, limit: Duration, pred: impl Fn(&Rec) -> bool) -> Rec {
        if let Some(found) = self.all.iter().find(|r| pred(r)) {
            return found.clone();
        }
        tokio::time::timeout(limit, async {
            loop {
                let rec = self.rx.recv().await.expect("recorder closed");
                self.all.push(rec.clone());
                if pred(&rec) {
                    return rec;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out; saw {:#?}", self.all))
    }
    async fn until(&mut self, pred: impl Fn(&Rec) -> bool) -> Rec {
        self.until_within(WAIT, pred).await
    }
    async fn link(&mut self, state: LinkState) -> DaemonView {
        match self
            .until(|r| matches!(r, Rec::Link(v) if v.link == state))
            .await
        {
            Rec::Link(view) => *view,
            _ => unreachable!(),
        }
    }
    /// Takes whatever is already queued without waiting.
    fn drain(&mut self) {
        while let Ok(rec) = self.rx.try_recv() {
            self.all.push(rec);
        }
    }
    fn links(&self) -> Vec<LinkState> {
        self.all
            .iter()
            .filter_map(|r| match r {
                Rec::Link(v) => Some(v.link),
                _ => None,
            })
            .collect()
    }
}

fn recorder(log_delay: Duration) -> (Arc<Recorder>, Seen) {
    let (tx, rx) = unbounded_channel();
    (
        Arc::new(Recorder { tx, log_delay }),
        Seen {
            rx,
            all: Vec::new(),
        },
    )
}

fn fast() -> RelinkPolicy {
    RelinkPolicy {
        delays: vec![Duration::from_millis(20)],
    }
}

fn start(socket: &Path, sink: Arc<Recorder>, policy: RelinkPolicy) -> SupervisorHandle {
    spawn(socket.to_path_buf(), "0.0.1".into(), sink, policy)
}

/// Polls `check` every few milliseconds until it holds, at most `WAIT`.
async fn eventually(what: &str, check: impl Fn() -> bool) {
    tokio::time::timeout(WAIT, async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

fn connected_state(rec: &Rec, wanted: State) -> bool {
    matches!(rec, Rec::State(s) if s.state == wanted)
}

#[tokio::test]
async fn first_views_are_connecting_then_connected() {
    let daemon = FakeDaemon::start(vec![], info(vec![])).await;
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&daemon.socket, sink, fast());
    seen.link(LinkState::Connected).await;
    assert_eq!(
        &seen.links()[..2],
        &[LinkState::Connecting, LinkState::Connected]
    );
    handle.shutdown().await;
    daemon.stop().await;
}

#[tokio::test]
async fn connected_view_carries_hello_status() {
    let daemon = FakeDaemon::start(vec![], info(vec![])).await;
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&daemon.socket, sink, fast());
    let view = seen.link(LinkState::Connected).await;
    assert_eq!(view.status.unwrap().state, State::Disconnected);
    assert_eq!(view.daemon_version.as_deref(), Some("0.1.0"));
    handle.shutdown().await;
    daemon.stop().await;
}

#[tokio::test]
async fn state_events_are_forwarded_in_order() {
    let daemon = FakeDaemon::start(vec![online()], info(vec![])).await;
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&daemon.socket, sink, fast());
    seen.link(LinkState::Connected).await;
    let mut other = daemon.client().await;
    let _: galleria_ipc::ConnectResult = other.call("connect", &params(false)).await.unwrap();
    seen.until(|r| connected_state(r, State::Connected)).await;
    let position = |wanted| seen.all.iter().position(|r| connected_state(r, wanted));
    assert!(position(State::Connecting).unwrap() < position(State::Connected).unwrap());
    eventually("the snapshot to follow the events", || {
        handle
            .view()
            .status
            .is_some_and(|s| s.state == State::Connected)
    })
    .await;
    handle.shutdown().await;
    daemon.stop().await;
}

#[tokio::test]
async fn stats_and_logs_arrive_after_subscribe() {
    let mut script = online();
    script.extend([
        FakeStep::Emit(FakeEvent::Log(LogLine {
            level: LogLevel::Info,
            message: "backend message".into(),
        })),
        FakeStep::Emit(FakeEvent::Stats(Stats {
            rx_bytes: 123,
            tx_bytes: 456,
            last_handshake: None,
        })),
    ]);
    let daemon = FakeDaemon::start(vec![script], info(vec![])).await;
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&daemon.socket, sink, fast());
    seen.link(LinkState::Connected).await;
    let mut other = daemon.client().await;
    let _: galleria_ipc::ConnectResult = other.call("connect", &params(false)).await.unwrap();
    seen.until(|r| matches!(r, Rec::Stats(s) if s.rx_bytes == 123))
        .await;
    seen.until(|r| matches!(r, Rec::Log(e) if e.message == "backend message"))
        .await;
    handle.shutdown().await;
    daemon.stop().await;
}

#[tokio::test]
async fn daemon_restart_relinks_without_user_action() {
    let dir = tempfile::tempdir().unwrap();
    let first = FakeDaemon::start_in(dir.path(), Harness::new(vec![]), info(vec![])).await;
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&first.socket, sink, fast());
    seen.link(LinkState::Connected).await;
    first.stop().await;
    seen.link(LinkState::DaemonNotRunning).await;
    let second = FakeDaemon::start_in(dir.path(), Harness::new(vec![]), info(vec![])).await;
    eventually("the link to come back", || {
        handle.view().link == LinkState::Connected
    })
    .await;
    assert_eq!(
        seen.links().last(),
        Some(&LinkState::DaemonNotRunning),
        "nothing else is emitted while the daemon is away"
    );
    handle.shutdown().await;
    second.stop().await;
}

#[tokio::test]
async fn repeated_failures_emit_one_link_event() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&dir.path().join("galleriad.sock"), sink, fast());
    eventually("ten attempts", || handle.attempts() >= 10).await;
    seen.link(LinkState::DaemonNotRunning).await;
    seen.drain();
    assert_eq!(
        seen.links()
            .iter()
            .filter(|l| **l == LinkState::DaemonNotRunning)
            .count(),
        1
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn retry_now_skips_the_wait() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, mut seen) = recorder(Duration::ZERO);
    let long_wait = RelinkPolicy {
        delays: vec![Duration::from_secs(60)],
    };
    let handle = start(&dir.path().join("galleriad.sock"), sink, long_wait);
    seen.link(LinkState::DaemonNotRunning).await;
    let daemon = FakeDaemon::start_in(dir.path(), Harness::new(vec![]), info(vec![])).await;
    handle.retry_now();
    seen.until_within(
        Duration::from_secs(2),
        |r| matches!(r, Rec::Link(v) if v.link == LinkState::Connected),
    )
    .await;
    handle.shutdown().await;
    daemon.stop().await;
}

/// Answers `hello` with the incompatible-protocol error and closes, as a daemon of another protocol would.
async fn incompatible_hello(stream: UnixStream) {
    let mut reader = BufReader::new(stream);
    let line = read_frame(&mut reader, MAX_MESSAGE_BYTES)
        .await
        .unwrap()
        .unwrap();
    let request: serde_json::Value = serde_json::from_slice(&line).unwrap();
    let reply = json!({"jsonrpc":"2.0","id":request["id"],"error":{
        "code":-32000,"message":"Incompatible protocol; update Galleria or restart the daemon",
        "data":{"code":"incompatible_protocol","daemon_min":2,"daemon_max":3,"daemon_version":"0.2.0"}}});
    reader
        .get_mut()
        .write_all(&encode_line(&reply))
        .await
        .unwrap();
}

#[tokio::test]
async fn incompatible_daemon_is_reported_and_retried() {
    let dir = tempfile::tempdir().unwrap();
    let scripted = Scripted::listen(dir.path(), incompatible_hello);
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&scripted.socket, sink, fast());
    let view = seen.link(LinkState::IncompatibleProtocol).await;
    let incompatible = view.incompatible.unwrap();
    assert_eq!(
        (
            incompatible.daemon_min,
            incompatible.daemon_max,
            incompatible.daemon_version.as_str()
        ),
        (2, 3, "0.2.0")
    );
    drop(scripted); // the socket goes away; a real daemon takes its place
    let daemon = FakeDaemon::start_in(dir.path(), Harness::new(vec![]), info(vec![])).await;
    seen.link(LinkState::Connected).await;
    handle.shutdown().await;
    daemon.stop().await;
}

#[tokio::test]
async fn slow_sink_does_not_stall_the_event_connection() {
    let mut script = online();
    for i in 0..50 {
        script.push(FakeStep::Emit(FakeEvent::Log(LogLine {
            level: LogLevel::Info,
            message: format!("line {i}"),
        })));
    }
    script.push(FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.9"))));
    let daemon = FakeDaemon::start(vec![script], info(vec![])).await;
    let (sink, mut seen) = recorder(Duration::from_millis(200));
    let handle = start(&daemon.socket, sink, fast());
    seen.link(LinkState::Connected).await;
    let mut other = daemon.client().await;
    let _: galleria_ipc::ConnectResult = other.call("connect", &params(false)).await.unwrap();
    // The sink needs about ten seconds for the log lines. The supervisor must have read the last state event
    // long before that: its snapshot is updated by the reading loop, not by the sink.
    tokio::time::timeout(Duration::from_secs(3), async {
        while !handle.view().status.is_some_and(|s| {
            s.net
                .is_some_and(|n| n.dns_servers.iter().any(|d| d.to_string() == "10.0.0.9"))
        }) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the supervisor stalled behind the slow sink");
    seen.until_within(
        Duration::from_secs(30),
        |r| matches!(r, Rec::Log(e) if e.message == "line 49"),
    )
    .await;
    let lines: Vec<u64> = seen
        .all
        .iter()
        .filter_map(|r| match r {
            Rec::Log(e) if e.message.starts_with("line ") => Some(e.seq),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 50);
    assert!(
        lines.windows(2).all(|w| w[0] < w[1]),
        "out of order: {lines:?}"
    );
    assert!(
        !seen.links().contains(&LinkState::DaemonNotRunning),
        "the daemon dropped the connection: {:?}",
        seen.links()
    );
    handle.shutdown().await;
    daemon.stop().await;
}

/// Answers `hello` and `subscribe` like a daemon and reports when the client closes the connection.
async fn hello_and_subscribe_until_eof(stream: UnixStream, closed: UnboundedSender<()>) {
    let mut reader = BufReader::new(stream);
    while let Ok(Some(line)) = read_frame(&mut reader, MAX_MESSAGE_BYTES).await {
        let request: serde_json::Value = serde_json::from_slice(&line).unwrap();
        let result = match request["method"].as_str().unwrap() {
            "hello" => serde_json::to_value(HelloResult {
                protocol: 1,
                daemon_version: "0.1.0".into(),
                backends: vec![],
                status: Status::default(),
            })
            .unwrap(),
            _ => json!({}),
        };
        let reply = json!({"jsonrpc":"2.0","id":request["id"],"result":result});
        reader
            .get_mut()
            .write_all(&encode_line(&reply))
            .await
            .unwrap();
    }
    let _ = closed.send(());
}

#[tokio::test]
async fn shutdown_closes_the_connection() {
    let dir = tempfile::tempdir().unwrap();
    let (closed_tx, mut closed_rx) = unbounded_channel();
    let scripted = Scripted::listen(dir.path(), move |stream| {
        hello_and_subscribe_until_eof(stream, closed_tx.clone())
    });
    let (sink, mut seen) = recorder(Duration::ZERO);
    let handle = start(&scripted.socket, sink, fast());
    seen.link(LinkState::Connected).await;
    handle.shutdown().await;
    tokio::time::timeout(Duration::from_secs(2), closed_rx.recv())
        .await
        .expect("the scripted server never saw the connection close")
        .unwrap();
}
