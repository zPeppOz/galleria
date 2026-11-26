//! The window of the tests of the connection: a mock Tauri app over a temporary rubrica, with the fake daemon of a scenario,
//! the commands invoked as the webview does and the events heard with a Rust listener. Shared by `tests/connection.rs` and
//! `tests/credentials.rs`.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use crate::{
    seed,
    support::{scenarios::scenario, *},
};
use galleria_gui::{
    announce::{Announcer, Urgency},
    configure_with,
    export::SavePicker,
    rubrica::{Rubrica, SecretFactory},
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    Listener, WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};
use uuid::Uuid;

pub const WAIT: Duration = Duration::from_secs(8);
pub const EVENTS: [&str; 4] = [
    "galleria:link",
    "galleria:state",
    "galleria:stats",
    "galleria:log",
];

#[derive(Clone, Default)]
pub struct Recorder(pub Arc<Mutex<Vec<(String, Urgency)>>>);

impl Announcer for Recorder {
    fn announce(&self, text: &str, urgency: Urgency) {
        self.0.lock().unwrap().push((text.to_owned(), urgency));
    }
}

pub struct World {
    pub rt: tokio::runtime::Runtime,
    pub _app: tauri::App<MockRuntime>,
    pub window: WebviewWindow<MockRuntime>,
    pub daemon: Option<FakeDaemon>,
    pub root: tempfile::TempDir,
    pub rubrica: Arc<Rubrica>,
    pub seeded: seed::Seeded,
    pub spoken: Recorder,
    pub events: Arc<Mutex<Vec<(String, Value)>>>,
}

pub struct Options {
    pub scenario: &'static str,
    pub daemon: bool,
    pub secrets: Option<SecretFactory>,
    /// The window «Save» of the test; without one the app keeps its own (which, in a mock app, has no window to ask).
    pub picker: Option<Arc<dyn SavePicker>>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scenario: "plain",
            daemon: true,
            secrets: None,
            picker: None,
        }
    }
}

pub fn start(options: Options) -> World {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("galleria");
    let seeded = seed::seed(&dir, false);
    let state = Arc::new(UiStateFile::new(
        root.path().join("state/galleria/state.toml"),
    ));
    let rubrica = Arc::new(match options.secrets {
        Some(factory) => Rubrica::with_secrets(dir, Arc::clone(&state), factory),
        None => Rubrica::with_file_secrets(dir, Arc::clone(&state)),
    });
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let sock_dir = root.path().join("run");
    fs::create_dir_all(&sock_dir).unwrap();
    let spoken = Recorder::default();
    let announcer: Arc<dyn Announcer> = Arc::new(spoken.clone());
    let mut builder = mock_builder()
        .manage(Arc::clone(&rubrica))
        .manage(announcer);
    if let Some(picker) = options.picker {
        builder = builder.manage(picker);
    }
    let mut app = configure_with(
        builder,
        sock_dir.join("galleriad.sock"),
        RelinkPolicy {
            delays: vec![Duration::from_millis(20)],
        },
        UiStateFile::new(root.path().join("state/galleria/state.toml")),
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let events = Arc::new(Mutex::new(Vec::new()));
    for name in EVENTS {
        let sink = Arc::clone(&events);
        app.listen(name, move |e| {
            sink.lock()
                .unwrap()
                .push((name.to_owned(), serde_json::from_str(e.payload()).unwrap()));
        });
    }
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let daemon = options.daemon.then(|| {
        let scripts = scenario(options.scenario).unwrap();
        rt.block_on(async {
            FakeDaemon::start_in(&sock_dir, Harness::new(scripts), info(vec![])).await
        })
    });
    let world = World {
        rt,
        _app: app,
        window,
        daemon,
        root,
        rubrica,
        seeded,
        spoken,
        events,
    };
    if world.daemon.is_some() {
        let begun = Instant::now();
        while world.invoke("daemon_view", json!({})).unwrap()["link"] != "connected" {
            assert!(
                begun.elapsed() < WAIT,
                "the link never came up: {}; events {:#?}",
                world.invoke("daemon_view", json!({})).unwrap(),
                world.events.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    world
}

impl World {
    pub fn id(&self, name: &str) -> Uuid {
        self.seeded
            .profiles
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, id)| *id)
            .unwrap()
    }

    pub fn daemon(&self) -> &FakeDaemon {
        self.daemon.as_ref().unwrap()
    }

    pub fn invoke(&self, cmd: &str, body: Value) -> Result<Value, Value> {
        get_ipc_response(
            &self.window,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|body| body.deserialize::<Value>().unwrap())
    }

    pub fn connect(&self, name: &str, replace: bool) -> Result<Value, Value> {
        self.invoke(
            "connect_profile",
            json!({ "id": self.id(name), "replace": replace }),
        )
    }

    /// Waits until what the events have brought satisfies `pred`, at most `WAIT`.
    pub fn until(&self, pred: impl Fn(&[(String, Value)]) -> bool) {
        let start = Instant::now();
        loop {
            if pred(&self.events.lock().unwrap()) {
                return;
            }
            assert!(
                start.elapsed() < WAIT,
                "the events never got there; saw {:#?}",
                self.events.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn until_state(&self, state: &str, profile: Option<Uuid>) {
        self.until(|events| {
            events.iter().any(|(name, value)| {
                name == "galleria:state"
                    && value["state"] == state
                    && profile.is_none_or(|id| value["profile"]["id"] == id.to_string())
            })
        });
    }

    /// The states the page was given, in order, without the repeats.
    pub fn states(&self) -> Vec<String> {
        let mut seen: Vec<String> = vec![];
        for (name, value) in self.events.lock().unwrap().iter() {
            if name == "galleria:state" {
                let state = value["state"].as_str().unwrap().to_owned();
                if seen.last() != Some(&state) {
                    seen.push(state);
                }
            }
        }
        seen
    }

    pub fn state_file(&self) -> String {
        fs::read_to_string(self.root.path().join("state/galleria/state.toml")).unwrap_or_default()
    }

    pub fn secrets_dir(&self) -> PathBuf {
        self.root.path().join("galleria")
    }

    pub fn phrases(&self) {
        self.invoke(
            "announce_phrases",
            json!({ "phrases": {
                "connected": "Connected to {name}",
                "disconnected": "Disconnected",
                "reconnecting": "Reconnecting to {name}",
                "error": { "server_unreachable": "Cannot reach the server of {name}" },
                "error_other": "Connection error",
            }}),
        )
        .unwrap();
    }

    pub fn stop(mut self) {
        if let Some(daemon) = self.daemon.take() {
            self.rt.block_on(daemon.stop());
        }
    }
}

pub fn code(error: &Value) -> &str {
    error["code"].as_str().unwrap()
}
