//! Connection state of the GUI towards the daemon and the classification of a failed attempt (spec §5, D27).
//! Pure Rust and tokio: the app framework is not imported here.

use galleria_ipc::{
    BackendInfo, DEFAULT_SOCKET_PATH, HelloResult, Status,
    client::{Client, ClientError},
};
use serde::Serialize;
use std::{
    error::Error as _,
    ffi::OsString,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// Where the GUI stands with the daemon. The webview shows the first-run panel for every value but `Connected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    Connecting,
    Connected,
    DaemonNotRunning,
    PermissionDenied,
    IncompatibleProtocol,
}

/// The protocol range and version a daemon reported in an `incompatible_protocol` answer to `hello`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Incompatible {
    pub daemon_min: u32,
    pub daemon_max: u32,
    pub daemon_version: String,
}

/// What the webview sees of the daemon: the last snapshot kept by the supervisor.
#[derive(Debug, Clone, Serialize)]
pub struct DaemonView {
    pub link: LinkState,
    pub daemon_version: Option<String>,
    pub incompatible: Option<Incompatible>,
    pub backends: Vec<BackendInfo>,
    pub status: Option<Status>,
}

impl DaemonView {
    /// The view before the first attempt has an answer.
    pub fn connecting() -> Self {
        Self {
            link: LinkState::Connecting,
            daemon_version: None,
            incompatible: None,
            backends: Vec::new(),
            status: None,
        }
    }

    /// The view after a successful `hello`: it carries the daemon's own `status`, so the webview never sees an
    /// empty state between the link coming up and the first event.
    pub fn connected(hello: &HelloResult) -> Self {
        Self {
            link: LinkState::Connected,
            daemon_version: Some(hello.daemon_version.clone()),
            incompatible: None,
            backends: hello.backends.clone(),
            status: Some(hello.status.clone()),
        }
    }

    /// The view after a failed attempt.
    pub fn failed(failure: &LinkFailure) -> Self {
        Self {
            link: failure.state,
            incompatible: failure.incompatible.clone(),
            ..Self::connecting()
        }
    }
}

/// A failed connection attempt, classified. `detail` is the error text for the GUI log (stderr): it never leaves
/// through a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFailure {
    pub state: LinkState,
    pub incompatible: Option<Incompatible>,
    pub detail: String,
}

/// Maps the outcome of connecting and saying `hello` to a link state (the table of spec §5).
pub fn classify(err: &ClientError) -> LinkFailure {
    let (state, incompatible) = match err {
        // The kernel refused the socket, or the daemon closed without answering `hello`: its second authorization
        // level (spec F1 §5.1). Both mean the user is outside the group.
        ClientError::PermissionDenied | ClientError::Closed => (LinkState::PermissionDenied, None),
        ClientError::Io(e) if e.kind() == ErrorKind::PermissionDenied => {
            (LinkState::PermissionDenied, None)
        }
        ClientError::Rpc(e) => match e.data.as_ref().and_then(incompatible_from) {
            Some(range) => (LinkState::IncompatibleProtocol, Some(range)),
            None => (LinkState::DaemonNotRunning, None),
        },
        // No socket file, nobody listening, `too_many_clients`, anything else: the daemon is not usable now and
        // the supervisor tries again.
        ClientError::Io(_) => (LinkState::DaemonNotRunning, None),
    };
    LinkFailure {
        state,
        incompatible,
        detail: describe(err),
    }
}

fn incompatible_from(data: &serde_json::Value) -> Option<Incompatible> {
    if data.get("code")?.as_str()? != "incompatible_protocol" {
        return None;
    }
    Some(Incompatible {
        daemon_min: u32::try_from(data.get("daemon_min")?.as_u64()?).ok()?,
        daemon_max: u32::try_from(data.get("daemon_max")?.as_u64()?).ok()?,
        daemon_version: data.get("daemon_version")?.as_str()?.to_owned(),
    })
}

fn describe(err: &ClientError) -> String {
    match err.source() {
        Some(source) => format!("{err}: {source}"),
        None => err.to_string(),
    }
}

/// Connects to the daemon and introduces itself as `galleria-gui`. On success the open client is returned with
/// the answer to `hello`; the caller keeps the connection.
///
/// # Errors
///
/// A [`LinkFailure`] whose `state` says which first-run panel applies.
pub async fn attempt(
    socket: &Path,
    client_version: &str,
) -> Result<(Client, HelloResult), LinkFailure> {
    let mut client = Client::connect(socket).await.map_err(|e| classify(&e))?;
    let hello = client
        .hello("galleria-gui", client_version)
        .await
        .map_err(|e| classify_hello(&e))?;
    Ok((client, hello))
}

/// Like [`classify`], for an error during `hello`, after `connect()` succeeded. The daemon refuses a peer outside
/// its group right after `accept`, without reading the request, so the client sees a clean close only if its read
/// comes first; if its write comes first, or the kernel resets the connection because unread bytes were left, it
/// sees a broken pipe or a reset. After a successful `connect()` the three mean the same: permission denied.
fn classify_hello(err: &ClientError) -> LinkFailure {
    match err {
        ClientError::Io(e)
            if matches!(e.kind(), ErrorKind::BrokenPipe | ErrorKind::ConnectionReset) =>
        {
            LinkFailure {
                state: LinkState::PermissionDenied,
                incompatible: None,
                detail: describe(err),
            }
        }
        _ => classify(err),
    }
}

/// The daemon socket: `GALLERIA_SOCKET` when it is set and not empty, the default path otherwise. The caller
/// reads the environment; this stays pure so tests can pass a value.
pub fn socket_path(env_override: Option<OsString>) -> PathBuf {
    match env_override {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => PathBuf::from(DEFAULT_SOCKET_PATH),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use galleria_ipc::{RpcError, client::ClientError};
    use rstest::rstest;
    use serde_json::json;
    use std::io::{self, ErrorKind};

    fn io(kind: ErrorKind) -> ClientError {
        ClientError::Io(io::Error::from(kind))
    }
    fn rpc(code: &str) -> ClientError {
        ClientError::Rpc(RpcError {
            code: -32000,
            message: "x".into(),
            data: Some(json!({ "code": code })),
        })
    }

    #[rstest]
    #[case::no_socket(io(ErrorKind::NotFound), LinkState::DaemonNotRunning)]
    #[case::nobody_listening(io(ErrorKind::ConnectionRefused), LinkState::DaemonNotRunning)]
    #[case::kernel_refused(ClientError::PermissionDenied, LinkState::PermissionDenied)]
    #[case::closed_before_reply(ClientError::Closed, LinkState::PermissionDenied)]
    #[case::too_many_clients(rpc("too_many_clients"), LinkState::DaemonNotRunning)]
    #[case::other_io(io(ErrorKind::InvalidData), LinkState::DaemonNotRunning)]
    fn classify_follows_the_spec_table(#[case] err: ClientError, #[case] expected: LinkState) {
        assert_eq!(classify(&err).state, expected);
    }

    #[rstest]
    #[case::broken_pipe(io(ErrorKind::BrokenPipe), LinkState::PermissionDenied)]
    #[case::reset(io(ErrorKind::ConnectionReset), LinkState::PermissionDenied)]
    #[case::closed(ClientError::Closed, LinkState::PermissionDenied)]
    #[case::refused(io(ErrorKind::ConnectionRefused), LinkState::DaemonNotRunning)]
    #[case::too_many_clients(rpc("too_many_clients"), LinkState::DaemonNotRunning)]
    fn an_error_during_hello_after_a_connect_follows_the_same_table(
        #[case] err: ClientError,
        #[case] expected: LinkState,
    ) {
        assert_eq!(classify_hello(&err).state, expected);
    }

    #[test]
    fn a_broken_pipe_outside_hello_is_still_daemon_not_running() {
        // `connect()` errors keep their classification: a reset there is not a refusal by the daemon.
        assert_eq!(
            classify(&io(ErrorKind::BrokenPipe)).state,
            LinkState::DaemonNotRunning
        );
    }

    #[test]
    fn incompatible_protocol_carries_the_daemon_range() {
        let err = ClientError::Rpc(RpcError {
            code: -32000,
            message: "x".into(),
            data: Some(
                json!({"code":"incompatible_protocol","daemon_min":2,"daemon_max":3,"daemon_version":"0.2.0"}),
            ),
        });
        let failure = classify(&err);
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
}
