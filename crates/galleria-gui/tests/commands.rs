#![cfg(unix)]
//! Level-2 tests of the Tauri commands and events, on the mock runtime of `tauri::test` (S4) and against a real
//! daemon with a fake backend. The webview is simulated: commands are invoked through the same entry point the
//! webview uses, events are heard with a Rust listener.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;

use galleria_gui::{
    configure_with,
    error::CmdError,
    events::{EV_LINK, EV_STATE},
    supervisor::RelinkPolicy,
};
use galleria_ipc::ConnectResult;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::mpsc::{Receiver, channel},
    time::{Duration, Instant},
};
use support::*;
use tauri::{
    Listener, WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};

const WAIT: Duration = Duration::from_secs(5);

/// The app on the mock runtime with its main window, plus the runtime the fake daemon runs on.
struct Gui {
    rt: tokio::runtime::Runtime,
    app: tauri::App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
}

fn gui(socket: &Path, policy: RelinkPolicy) -> Gui {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    // `state.toml` goes next to the socket, in the temporary directory of the test: never in the home of whoever runs it.
    let state = galleria_gui::ui_state::UiStateFile::new(socket.with_file_name("state/state.toml"));
    let mut app = configure_with(mock_builder(), socket.to_path_buf(), policy, state)
        .build(mock_context(noop_assets()))
        .unwrap();
    // `build` does not run the setup hook on the mock runtime; one iteration does (S4: found in B1-T4).
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    Gui { rt, app, window }
}

/// A fake daemon on the test's own runtime, with its socket in `dir`.
fn start_daemon(
    gui: &Gui,
    dir: &Path,
    scripts: Vec<Vec<galleria_backend::fake::FakeStep>>,
) -> FakeDaemon {
    gui.rt
        .block_on(async { FakeDaemon::start_in(dir, Harness::new(scripts), info(vec![])).await })
}

fn fast() -> RelinkPolicy {
    RelinkPolicy {
        delays: vec![Duration::from_millis(20)],
    }
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::default(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

/// Invokes `daemon_view` until its `link` is `wanted`, at most `limit`.
fn view_reaches(window: &WebviewWindow<MockRuntime>, wanted: &str, limit: Duration) -> Value {
    let start = Instant::now();
    loop {
        let view = invoke(window, "daemon_view").unwrap();
        if view["link"] == wanted {
            return view;
        }
        assert!(
            start.elapsed() < limit,
            "daemon_view never reached {wanted}; last: {view}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn payloads(app: &tauri::App<MockRuntime>, event: &str) -> Receiver<Value> {
    let (tx, rx) = channel();
    app.listen(event, move |e| {
        let _ = tx.send(serde_json::from_str::<Value>(e.payload()).unwrap());
    });
    rx
}

fn next_where(rx: &Receiver<Value>, pred: impl Fn(&Value) -> bool) -> Value {
    let start = Instant::now();
    loop {
        let left = WAIT.saturating_sub(start.elapsed());
        let value = rx
            .recv_timeout(left)
            .expect("the expected event never arrived");
        if pred(&value) {
            return value;
        }
    }
}

#[test]
fn daemon_view_is_connected_against_the_fake_daemon() {
    let dir = tempfile::tempdir().unwrap();
    let gui = gui(&dir.path().join("galleriad.sock"), fast());
    let daemon = start_daemon(&gui, dir.path(), vec![]);
    let view = view_reaches(&gui.window, "connected", WAIT);
    assert_eq!(view["status"]["state"], "disconnected");
    gui.rt.block_on(daemon.stop());
}

#[test]
fn daemon_view_without_daemon_is_daemon_not_running() {
    let dir = tempfile::tempdir().unwrap();
    let gui = gui(&dir.path().join("galleriad.sock"), fast());
    view_reaches(&gui.window, "daemon_not_running", WAIT);
}

#[test]
fn daemon_retry_relinks_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let long_wait = RelinkPolicy {
        delays: vec![Duration::from_secs(60)],
    };
    let gui = gui(&dir.path().join("galleriad.sock"), long_wait);
    view_reaches(&gui.window, "daemon_not_running", WAIT);
    let daemon = start_daemon(&gui, dir.path(), vec![]);
    invoke(&gui.window, "daemon_retry").unwrap();
    view_reaches(&gui.window, "connected", Duration::from_secs(2));
    gui.rt.block_on(daemon.stop());
}

#[test]
fn app_info_reports_user_group_and_socket() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("galleriad.sock");
    let gui = gui(&socket, fast());
    let info = invoke(&gui.window, "app_info").unwrap();
    let user = nix::unistd::User::from_uid(nix::unistd::getuid())
        .unwrap()
        .unwrap()
        .name;
    assert_eq!(info["user"], user.as_str());
    assert_eq!(info["group"], "galleria");
    assert_eq!(info["socket_path"], socket.to_str().unwrap());
    assert_eq!(info["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn state_event_reaches_a_listener() {
    let dir = tempfile::tempdir().unwrap();
    let gui = gui(&dir.path().join("galleriad.sock"), fast());
    let states = payloads(&gui.app, EV_STATE);
    let daemon = start_daemon(&gui, dir.path(), vec![online()]);
    view_reaches(&gui.window, "connected", WAIT);
    gui.rt.block_on(async {
        let mut other = daemon.client().await;
        let _: ConnectResult = other.call("connect", &params(false)).await.unwrap();
    });
    next_where(&states, |s| s["state"] == "connecting");
    gui.rt.block_on(daemon.stop());
}

#[test]
fn link_event_reaches_a_listener() {
    let dir = tempfile::tempdir().unwrap();
    let gui = gui(&dir.path().join("galleriad.sock"), fast());
    let links = payloads(&gui.app, EV_LINK);
    let daemon = start_daemon(&gui, dir.path(), vec![]);
    next_where(&links, |v| v["link"] == "connected");
    gui.rt.block_on(daemon.stop());
    next_where(&links, |v| v["link"] == "daemon_not_running");
}

fn keys_of(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                out.push(key.clone());
                keys_of(inner, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys_of(item, out)),
        _ => {}
    }
}

#[test]
fn command_outputs_never_contain_secret_fields() {
    const SECRET_KEYS: [&str; 7] = [
        "private_key",
        "preshared_key",
        "password",
        "otp",
        "key_passphrase",
        "secret_blocks",
        "config",
    ];
    let dir = tempfile::tempdir().unwrap();
    let gui = gui(&dir.path().join("galleriad.sock"), fast());
    let daemon = start_daemon(&gui, dir.path(), vec![online()]);
    view_reaches(&gui.window, "connected", WAIT);
    // A connected session as well: the view then carries a full `status`.
    gui.rt.block_on(async {
        let mut other = daemon.client().await;
        let _: ConnectResult = other.call("connect", &params(false)).await.unwrap();
    });
    let start = Instant::now();
    while invoke(&gui.window, "daemon_view").unwrap()["status"]["state"] != "connected" {
        assert!(start.elapsed() < WAIT, "the session never connected");
        std::thread::sleep(Duration::from_millis(5));
    }
    for cmd in ["daemon_view", "app_info"] {
        let mut keys = Vec::new();
        keys_of(&invoke(&gui.window, cmd).unwrap(), &mut keys);
        for secret in SECRET_KEYS {
            assert!(
                !keys.iter().any(|k| k == secret),
                "{cmd} returns `{secret}`"
            );
        }
    }
    gui.rt.block_on(daemon.stop());
}

#[test]
fn cmd_error_serializes_code_and_message() {
    let error = CmdError {
        code: "gui_x".into(),
        message: "y".into(),
        data: None,
    };
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        json!({"code":"gui_x","message":"y"})
    );
}
