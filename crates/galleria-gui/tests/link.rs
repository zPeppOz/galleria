#![cfg(unix)]
//! Level-2 tests of the daemon link against a real daemon with a fake backend and against scripted servers.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;

use galleria_gui::link::{DaemonView, Incompatible, LinkFailure, LinkState, attempt, socket_path};
use galleria_ipc::{
    BackendInfo, Capabilities, HelloResult, MAX_MESSAGE_BYTES, Protocol, State, client::Client,
    encode_line, read_frame,
};
use serde_json::json;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use support::*;
use tokio::{io::AsyncWriteExt, io::BufReader, net::UnixStream};

type Attempt = Result<(Client, HelloResult), LinkFailure>;

fn failure(result: Attempt) -> LinkFailure {
    match result {
        Ok(_) => panic!("the attempt succeeded"),
        Err(failure) => failure,
    }
}

/// Reads the `hello` request line, as a daemon that is listening does, and returns the stream with its request id.
async fn read_hello(stream: UnixStream) -> (BufReader<UnixStream>, serde_json::Value) {
    let mut reader = BufReader::new(stream);
    let line = read_frame(&mut reader, MAX_MESSAGE_BYTES)
        .await
        .unwrap()
        .unwrap();
    (reader, serde_json::from_slice(&line).unwrap())
}

fn unavailable_openvpn() -> BackendInfo {
    BackendInfo {
        protocol: Protocol::Openvpn,
        available: false,
        version: None,
        unavailable_reason: Some("openvpn_not_found".into()),
        capabilities: Capabilities {
            handshake: false,
            interactive_auth: false,
            server_pushed_config: false,
        },
    }
}

#[tokio::test]
async fn fake_daemon_gives_connected_with_disconnected_status() {
    let daemon = FakeDaemon::start(vec![], info(vec![])).await;
    let (_client, hello) = attempt(&daemon.socket, "0.0.1").await.ok().unwrap();
    assert_eq!(hello.status.state, State::Disconnected);
    daemon.stop().await;
}

#[tokio::test]
async fn missing_socket_is_daemon_not_running() {
    let dir = tempfile::tempdir().unwrap();
    let failure = failure(attempt(&dir.path().join("galleriad.sock"), "0.0.1").await);
    assert_eq!(failure.state, LinkState::DaemonNotRunning);
}

#[tokio::test]
async fn stale_socket_file_is_daemon_not_running() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("galleriad.sock");
    // Binding creates the file; dropping the listener closes it but leaves the file behind, like a dead daemon.
    drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
    assert!(socket.exists());
    assert_eq!(
        failure(attempt(&socket, "0.0.1").await).state,
        LinkState::DaemonNotRunning
    );
}

#[tokio::test]
async fn unreadable_socket_is_permission_denied() {
    // As root the mode bits do not stop a connection: this runs in the `gui` CI job (normal user), not in the image.
    if nix::unistd::geteuid().is_root() {
        eprintln!("skipped: running as root, file modes do not apply");
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let server = Scripted::listen(dir.path(), |_| async {});
    std::fs::set_permissions(&server.socket, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        failure(attempt(&server.socket, "0.0.1").await).state,
        LinkState::PermissionDenied
    );
}

#[tokio::test]
async fn daemon_closing_before_hello_reply_is_permission_denied() {
    let dir = tempfile::tempdir().unwrap();
    // A daemon that reads the request and closes without an answer: the client sees the close as `Closed`.
    let server = Scripted::listen(dir.path(), |stream| async move {
        let (reader, _) = read_hello(stream).await;
        drop(reader);
    });
    assert_eq!(
        failure(attempt(&server.socket, "0.0.1").await).state,
        LinkState::PermissionDenied
    );
}

/// The real daemon refuses a peer outside the group right after `accept`, without reading `hello` (F1 §5.1). The
/// client then sees a broken pipe or a reset, depending on whether its write or its read comes first, and both
/// mean the same as a clean close: the user is outside the group.
#[tokio::test]
async fn daemon_closing_right_after_accept_is_permission_denied() {
    let dir = tempfile::tempdir().unwrap();
    let server = Scripted::listen(dir.path(), |stream| async move { drop(stream) });
    for round in 0..200 {
        let failure = failure(attempt(&server.socket, "0.0.1").await);
        assert_eq!(
            failure.state,
            LinkState::PermissionDenied,
            "attempt {round}: {}",
            failure.detail
        );
    }
}

#[tokio::test]
async fn hello_out_of_range_is_incompatible_protocol() {
    let dir = tempfile::tempdir().unwrap();
    let server = Scripted::listen(dir.path(), |stream| async move {
        let (mut reader, request) = read_hello(stream).await;
        let reply = json!({"jsonrpc":"2.0","id":request["id"],"error":{
            "code":-32000,"message":"Incompatible protocol; update Galleria or restart the daemon",
            "data":{"code":"incompatible_protocol","daemon_min":2,"daemon_max":3,"daemon_version":"0.2.0"}}});
        reader
            .get_mut()
            .write_all(&encode_line(&reply))
            .await
            .unwrap();
    });
    let failure = failure(attempt(&server.socket, "0.0.1").await);
    assert_eq!(failure.state, LinkState::IncompatibleProtocol);
    assert_eq!(
        failure.incompatible,
        Some(Incompatible {
            daemon_min: 2,
            daemon_max: 3,
            daemon_version: "0.2.0".into()
        })
    );
}

#[tokio::test]
async fn unavailable_backend_is_kept_in_the_view() {
    let daemon = FakeDaemon::start(vec![], info(vec![unavailable_openvpn()])).await;
    let (_client, hello) = attempt(&daemon.socket, "0.0.1").await.ok().unwrap();
    let view = DaemonView::connected(&hello);
    assert_eq!(view.link, LinkState::Connected);
    assert_eq!(
        view.backends[0].unavailable_reason.as_deref(),
        Some("openvpn_not_found")
    );
    daemon.stop().await;
}

#[test]
fn view_serializes_with_snake_case_link() {
    assert_eq!(
        serde_json::to_value(DaemonView::connecting()).unwrap(),
        json!({"link":"connecting","daemon_version":null,"incompatible":null,"backends":[],"status":null})
    );
}

#[test]
fn socket_path_prefers_the_env_override() {
    assert_eq!(
        socket_path(Some(OsString::from("/tmp/x.sock"))),
        PathBuf::from("/tmp/x.sock")
    );
    assert_eq!(
        socket_path(None),
        Path::new(galleria_ipc::DEFAULT_SOCKET_PATH)
    );
}
