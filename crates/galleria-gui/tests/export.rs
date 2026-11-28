#![cfg(unix)]
//! Duplicate, export and delete (plan B3, T8): what is written, what is not, and that no secret and no path crosses the
//! command. The window «Save» is a `SavePicker` that the tests replace.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;
mod support;

use galleria_gui::{
    configure_with,
    export::{ExportError, SavePicker, suggested_name, write_export},
    rubrica::{Rubrica, RubricaError},
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use galleria_ipc::{ConnectParams, ConnectResult, Protocol, Secret};
use galleria_profile::{
    FileSecretStore, ProfileStore, SecretStore, StoreError, import_openvpn, import_wireguard,
};
use rstest::rstest;
use seed::{SECRET_MARK, seed};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use support::{FakeDaemon, Harness, info, online};
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};
use uuid::Uuid;

fn state_in(root: &Path) -> Arc<UiStateFile> {
    Arc::new(UiStateFile::new(root.join("state/galleria/state.toml")))
}

fn rubrica_in(root: &Path) -> (Rubrica, Arc<UiStateFile>) {
    let state = state_in(root);
    (
        Rubrica::with_file_secrets(root.join("galleria"), Arc::clone(&state)),
        state,
    )
}

fn id_of(seeded: &seed::Seeded, name: &str) -> Uuid {
    seeded.profiles.iter().find(|(n, _)| n == name).unwrap().1
}

fn profile_files(root: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(root.join("galleria/profiles"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn secret_files(root: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(root.join("galleria/secrets"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A window «Save» that answers what the test says and remembers the name it was offered.
struct FakePicker {
    answer: Option<PathBuf>,
    offered: Mutex<Vec<String>>,
}

impl FakePicker {
    fn new(answer: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            answer,
            offered: Mutex::new(vec![]),
        })
    }
}

impl SavePicker for FakePicker {
    fn pick(&self, suggested_name: &str) -> Option<PathBuf> {
        self.offered.lock().unwrap().push(suggested_name.to_owned());
        self.answer.clone()
    }
}

// ---- duplicate -------------------------------------------------------------------------------------------------------

#[test]
fn duplicate_copies_profile_and_secrets_under_a_new_id() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let original_id = id_of(&seeded, "Ufficio Napoli");
    let original = rubrica.get(original_id).unwrap();

    let copy = rubrica
        .duplicate(original_id, "  Ufficio Napoli (copy)  ")
        .unwrap();
    assert_ne!(copy.id, original_id);
    assert_eq!(copy.name, "Ufficio Napoli (copy)");
    assert!(!copy.favorite, "a copy is not a favourite");
    assert_eq!(copy.group, original.summary.group);
    assert_eq!(copy.protocol, original.summary.protocol);
    assert_eq!(copy.server, original.summary.server);

    let store = ProfileStore::open(&root.path().join("galleria")).unwrap();
    let (a, b) = (
        store.find(&original_id.to_string()).unwrap(),
        store.find(&copy.id.to_string()).unwrap(),
    );
    assert_eq!(a.name, "Ufficio Napoli", "the original is untouched");
    assert!(a.favorite);
    assert_eq!(
        a.openvpn.as_ref().unwrap().config,
        b.openvpn.as_ref().unwrap().config
    );
    assert_eq!(a.secret_refs, b.secret_refs);
    assert_eq!(a.settings.auto_reconnect, b.settings.auto_reconnect);
    assert_eq!(b.created, b.modified);
    assert_ne!(b.created, a.created);
    // the same secrets, under the new id
    let secrets = FileSecretStore::open(&root.path().join("galleria")).unwrap();
    assert!(!b.secret_refs.is_empty());
    for key in &b.secret_refs {
        let had = secrets.get(&original_id, key).unwrap().unwrap();
        let got = secrets.get(&copy.id, key).unwrap().unwrap();
        assert_eq!(had.expose(), got.expose(), "{key}");
    }
    // both files are private
    for id in [original_id, copy.id] {
        let mode = fs::metadata(root.path().join(format!("galleria/profiles/{id}.toml")))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        let mode = fs::metadata(root.path().join(format!("galleria/secrets/{id}.toml")))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
    assert_eq!(rubrica.list().unwrap().profiles.len(), 6);
}

#[test]
fn duplicate_copies_a_wireguard_profile_with_its_shared_key() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let original_id = id_of(&seeded, "Homelab");
    let copy = rubrica.duplicate(original_id, "Homelab 2").unwrap();
    let secrets = FileSecretStore::open(&root.path().join("galleria")).unwrap();
    for key in ["wireguard.private_key", "wireguard.peer.0.preshared_key"] {
        assert_eq!(
            secrets.get(&copy.id, key).unwrap().unwrap().expose(),
            secrets.get(&original_id, key).unwrap().unwrap().expose(),
            "{key}"
        );
    }
}

/// An archive of secrets that fails on the n-th write and remembers which ids were wiped.
struct Failing {
    inner: FileSecretStore,
    writes_left: Mutex<usize>,
    wiped: Arc<Mutex<Vec<Uuid>>>,
}

impl SecretStore for Failing {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError> {
        self.inner.get(id, key)
    }
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError> {
        let mut left = self.writes_left.lock().unwrap();
        if *left == 0 {
            return Err(StoreError::Io(std::io::ErrorKind::PermissionDenied.into()));
        }
        *left -= 1;
        self.inner.set(id, key, value)
    }
    fn delete(&self, id: &Uuid, key: &str) -> Result<(), StoreError> {
        self.inner.delete(id, key)
    }
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError> {
        self.wiped.lock().unwrap().push(*id);
        self.inner.delete_all(id)
    }
}

#[rstest]
#[case::the_first_secret(0)]
#[case::the_second_secret(1)]
fn duplicate_is_all_or_nothing(#[case] writable: usize) {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let state = state_in(root.path());
    let wiped = Arc::new(Mutex::new(vec![]));
    let seen = Arc::clone(&wiped);
    let rubrica = Rubrica::with_secrets(
        root.path().join("galleria"),
        state,
        Arc::new(move |dir: &Path, _name: &str| {
            Ok(Box::new(Failing {
                inner: FileSecretStore::open(dir)?,
                writes_left: Mutex::new(writable),
                wiped: Arc::clone(&seen),
            }) as Box<dyn SecretStore>)
        }),
    );
    let (profiles, secrets) = (profile_files(root.path()), secret_files(root.path()));

    // «Ufficio Napoli» keeps two secrets: the key and the tls-crypt block
    let error = rubrica
        .duplicate(id_of(&seeded, "Ufficio Napoli"), "Copia")
        .unwrap_err();
    assert!(matches!(error, RubricaError::Store { .. }), "{error:?}");
    assert_eq!(profile_files(root.path()), profiles, "no profile was left");
    assert_eq!(secret_files(root.path()), secrets, "no secret was left");
    assert_eq!(wiped.lock().unwrap().len(), 1, "what was written was wiped");
}

#[test]
fn duplicate_refuses_a_name_that_is_not_valid_and_writes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let before = (profile_files(root.path()), secret_files(root.path()));
    for (name, code) in [
        ("", "required"),
        ("   ", "required"),
        (&"x".repeat(65), "too_long"),
        ("a\nb", "invalid_chars"),
    ] {
        match rubrica
            .duplicate(id_of(&seeded, "Homelab"), name)
            .unwrap_err()
        {
            RubricaError::Invalid(fields) => {
                assert_eq!(
                    fields
                        .iter()
                        .map(|f| (f.field.as_str(), f.code))
                        .collect::<Vec<_>>(),
                    [("name", code)],
                    "{name:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(
        (profile_files(root.path()), secret_files(root.path())),
        before
    );
    // and a profile that is not there
    let error = rubrica.duplicate(Uuid::new_v4(), "Copia").unwrap_err();
    assert!(matches!(
        error,
        RubricaError::Store {
            source: StoreError::NotFound,
            ..
        }
    ));
}

#[test]
fn duplicate_fails_when_a_secret_of_the_original_is_missing() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Ufficio Napoli");
    fs::remove_file(root.path().join(format!("galleria/secrets/{id}.toml"))).unwrap();
    let before = (profile_files(root.path()), secret_files(root.path()));
    assert!(rubrica.duplicate(id, "Copia").is_err());
    assert_eq!(
        (profile_files(root.path()), secret_files(root.path())),
        before
    );
}

// ---- export ----------------------------------------------------------------------------------------------------------

#[test]
fn export_writes_a_private_file_at_the_picked_path() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let target = root.path().join("out/Ufficio.ovpn");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    let picker = FakePicker::new(Some(target.clone()));

    assert!(
        rubrica
            .export(id_of(&seeded, "Ufficio Napoli"), &*picker)
            .unwrap()
    );
    assert_eq!(
        *picker.offered.lock().unwrap(),
        ["Ufficio Napoli.ovpn".to_owned()]
    );
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let text = fs::read_to_string(&target).unwrap();
    // the file has the keys (Q31), and the import reads it back
    assert!(text.contains(SECRET_MARK) && text.contains("<tls-crypt>"));
    let again = import_openvpn(&text, "Ufficio Napoli", &|_| {
        Err(std::io::ErrorKind::NotFound.into())
    })
    .unwrap();
    assert_eq!(again.secrets.len(), 2);

    let target = root.path().join("out/Homelab.conf");
    let picker = FakePicker::new(Some(target.clone()));
    assert!(rubrica.export(id_of(&seeded, "Homelab"), &*picker).unwrap());
    assert_eq!(*picker.offered.lock().unwrap(), ["Homelab.conf".to_owned()]);
    let text = fs::read_to_string(&target).unwrap();
    assert!(text.contains("PrivateKey = ") && text.contains(SECRET_MARK));
    import_wireguard(&text, "Homelab").unwrap();
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn export_replaces_a_file_that_is_there_and_keeps_it_private() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("x.conf");
    fs::write(&target, "old").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
    write_export(&target, "new").unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    // no temporary file is left next to it
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn export_cancelled_writes_nothing_and_is_not_an_error() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let picker = FakePicker::new(None);
    assert!(!rubrica.export(id_of(&seeded, "Homelab"), &*picker).unwrap());
    assert_eq!(
        picker.offered.lock().unwrap().len(),
        1,
        "the window was shown"
    );
    // nothing but the rubrica itself is in the folder of the test: no file was written anywhere else
    let names: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["galleria"]);
}

#[test]
fn export_to_a_place_that_cannot_be_written_is_an_error_with_its_own_code() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let target = root.path().join("no/such/folder/x.conf");
    let picker = FakePicker::new(Some(target.clone()));
    let error = rubrica
        .export(id_of(&seeded, "Homelab"), &*picker)
        .unwrap_err();
    assert!(
        matches!(error, RubricaError::Export(ExportError::Io { .. })),
        "{error:?}"
    );
    assert!(!target.exists());
}

#[test]
fn export_fails_before_the_window_when_a_secret_is_missing() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Homelab");
    fs::remove_file(root.path().join(format!("galleria/secrets/{id}.toml"))).unwrap();
    let picker = FakePicker::new(Some(root.path().join("x.conf")));
    assert!(rubrica.export(id, &*picker).is_err());
    assert!(
        picker.offered.lock().unwrap().is_empty(),
        "no window for a file that cannot be made"
    );
    assert!(!root.path().join("x.conf").exists());
}

#[rstest]
#[case::plain("Ufficio Napoli", Protocol::Openvpn, "Ufficio Napoli.ovpn")]
#[case::wireguard("Homelab", Protocol::Wireguard, "Homelab.conf")]
#[case::a_slash("a/b", Protocol::Openvpn, "a_b.ovpn")]
#[case::a_backslash("a\\b", Protocol::Wireguard, "a_b.conf")]
#[case::dots("..", Protocol::Openvpn, "profile.ovpn")]
#[case::a_dot("   .   ", Protocol::Openvpn, "profile.ovpn")]
#[case::a_hidden_name("..hidden", Protocol::Wireguard, "hidden.conf")]
#[case::empty("", Protocol::Wireguard, "profile.conf")]
#[case::control_characters("a\nb\tc\u{7}", Protocol::Openvpn, "a_b_c_.ovpn")]
fn suggested_name_is_made_safe(#[case] name: &str, #[case] protocol: Protocol, #[case] want: &str) {
    assert_eq!(suggested_name(name, protocol), want);
}

#[test]
fn a_very_long_name_is_cut_and_keeps_its_extension() {
    let name = suggested_name(&"é".repeat(300), Protocol::Openvpn);
    assert!(name.ends_with(".ovpn"));
    assert!(
        name.chars().count() <= 100 + ".ovpn".len(),
        "{}",
        name.len()
    );
    assert!(
        name.len() <= 255,
        "a file name is at most 255 bytes: {}",
        name.len()
    );
    assert!(!name.contains('/'));
}

// ---- delete ----------------------------------------------------------------------------------------------------------

#[test]
fn delete_removes_profile_secrets_and_last_used() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let id = id_of(&seeded, "Ufficio Napoli");
    let other = id_of(&seeded, "Homelab");
    state
        .touch_last_used(id, time::OffsetDateTime::UNIX_EPOCH)
        .unwrap();
    state
        .touch_last_used(other, time::OffsetDateTime::UNIX_EPOCH)
        .unwrap();

    rubrica.delete(id, None).unwrap();
    assert!(
        !root
            .path()
            .join(format!("galleria/profiles/{id}.toml"))
            .exists()
    );
    assert!(
        !root
            .path()
            .join(format!("galleria/secrets/{id}.toml"))
            .exists()
    );
    assert!(!state.load().last_used.contains_key(&id));
    // the others are as they were
    assert!(state.load().last_used.contains_key(&other));
    assert_eq!(rubrica.list().unwrap().profiles.len(), 4);
    assert!(
        root.path()
            .join(format!("galleria/secrets/{other}.toml"))
            .exists()
    );

    let error = rubrica.delete(id, None).unwrap_err();
    assert!(
        matches!(
            error,
            RubricaError::Store {
                source: StoreError::NotFound,
                ..
            }
        ),
        "{error:?}"
    );
}

#[test]
fn delete_refuses_the_profile_of_the_active_session() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Homelab");
    let active = galleria_ipc::ProfileRef {
        id: id.to_string(),
        name: "Homelab".into(),
    };
    let error = rubrica.delete(id, Some(&active)).unwrap_err();
    assert!(matches!(error, RubricaError::InUse), "{error:?}");
    assert!(
        root.path()
            .join(format!("galleria/profiles/{id}.toml"))
            .exists()
    );
    assert!(
        root.path()
            .join(format!("galleria/secrets/{id}.toml"))
            .exists()
    );
    // another profile can go while this one is in use
    rubrica
        .delete(id_of(&seeded, "VPS Hetzner"), Some(&active))
        .unwrap();
}

// ---- the commands ----------------------------------------------------------------------------------------------------

struct Gui {
    rt: tokio::runtime::Runtime,
    _app: tauri::App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
}

fn gui(root: &Path, picker: Arc<FakePicker>, socket: &Path) -> Gui {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let (rubrica, state) = rubrica_in(root);
    let picker: Arc<dyn SavePicker> = picker;
    let mut app = configure_with(
        mock_builder().manage(Arc::new(rubrica)).manage(picker),
        socket.to_path_buf(),
        RelinkPolicy {
            delays: vec![Duration::from_millis(20)],
        },
        UiStateFile::new(state.path().to_path_buf()),
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    Gui {
        rt,
        _app: app,
        window,
    }
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        window,
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

#[test]
fn duplicate_returns_no_secret() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let gui = gui(
        root.path(),
        FakePicker::new(None),
        &root.path().join("none.sock"),
    );
    for name in ["Ufficio Napoli", "Homelab"] {
        let copy = invoke(
            &gui.window,
            "profile_duplicate",
            json!({ "id": id_of(&seeded, name).to_string(), "name": format!("{name} (copy)") }),
        )
        .unwrap();
        assert_eq!(copy["name"], format!("{name} (copy)"));
        assert_eq!(copy["favorite"], false);
        let shown = copy.to_string();
        assert!(
            !shown.contains(SECRET_MARK) && !shown.contains("PRIVATE KEY"),
            "{shown}"
        );
        // the copy is in the list, and the detail of it has no secret either
        let detail = invoke(&gui.window, "profile_get", json!({ "id": copy["id"] })).unwrap();
        assert!(!detail.to_string().contains(SECRET_MARK));
    }
    let error = invoke(
        &gui.window,
        "profile_duplicate",
        json!({ "id": id_of(&seeded, "Homelab").to_string(), "name": "" }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_invalid_field");
    assert_eq!(
        error["data"]["fields"],
        json!([{ "field": "name", "code": "required" }])
    );
}

#[test]
fn export_never_sends_the_content_to_the_webview() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let target = root.path().join("Ufficio.ovpn");
    let gui = gui(
        root.path(),
        FakePicker::new(Some(target.clone())),
        &root.path().join("none.sock"),
    );
    let answer = invoke(
        &gui.window,
        "profile_export",
        json!({ "id": id_of(&seeded, "Ufficio Napoli").to_string() }),
    )
    .unwrap();
    assert_eq!(answer, json!({ "saved": true }));
    assert!(
        fs::read_to_string(&target).unwrap().contains(SECRET_MARK),
        "the file has the keys"
    );

    // cancelled: `saved` is false and that is all
    let gui = self::gui(
        root.path(),
        FakePicker::new(None),
        &root.path().join("none2.sock"),
    );
    let answer = invoke(
        &gui.window,
        "profile_export",
        json!({ "id": id_of(&seeded, "Homelab").to_string() }),
    )
    .unwrap();
    assert_eq!(answer, json!({ "saved": false }));

    // a failure names a code and the place, never the text
    let gui = self::gui(
        root.path(),
        FakePicker::new(Some(root.path().join("no/such/x.conf"))),
        &root.path().join("none3.sock"),
    );
    let error = invoke(
        &gui.window,
        "profile_export",
        json!({ "id": id_of(&seeded, "Homelab").to_string() }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_export_failed");
    assert!(!error.to_string().contains(SECRET_MARK));
}

/// What the daemon is told to connect: the profile of the rubrica, by its id.
fn params_for(id: Uuid, name: &str) -> ConnectParams {
    serde_json::from_value(json!({
        "profile": { "id": id.to_string(), "name": name },
        "replace": false,
        "backend": { "protocol": "wireguard", "private_key": "synthetic", "addresses": [], "dns": [], "search_domains": [], "mtu": null, "peers": [] },
        "options": { "ipv6_block": true, "ignore_pushed_routes": false, "reconnect": { "enabled": true } }
    }))
    .unwrap()
}

#[test]
fn delete_refuses_the_profile_of_the_active_session_through_the_command() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let socket_dir = tempfile::tempdir().unwrap();
    let gui = gui(
        root.path(),
        FakePicker::new(None),
        &socket_dir.path().join("galleriad.sock"),
    );
    let daemon = gui.rt.block_on(async {
        FakeDaemon::start_in(
            socket_dir.path(),
            Harness::new(vec![online()]),
            info(vec![]),
        )
        .await
    });
    let homelab = id_of(&seeded, "Homelab");
    // The daemon is told to connect the profile of the rubrica, as the CLI would.
    gui.rt.block_on(async {
        let mut client = daemon.client().await;
        let _: ConnectResult = client
            .call("connect", &params_for(homelab, "Homelab"))
            .await
            .unwrap();
    });
    let start = Instant::now();
    loop {
        let view = invoke(&gui.window, "daemon_view", json!({})).unwrap();
        if view["status"]["state"] == "connected"
            && view["status"]["profile"]["id"] == homelab.to_string().as_str()
        {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the session never connected: {view}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    let error = invoke(
        &gui.window,
        "profile_delete",
        json!({ "id": homelab.to_string() }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_profile_in_use");
    assert!(
        root.path()
            .join(format!("galleria/profiles/{homelab}.toml"))
            .exists()
    );
    // another profile goes, and the answer has nothing in it
    let gone = id_of(&seeded, "Cliente Rossi");
    let answer = invoke(
        &gui.window,
        "profile_delete",
        json!({ "id": gone.to_string() }),
    )
    .unwrap();
    assert_eq!(answer, Value::Null);
    assert!(
        !root
            .path()
            .join(format!("galleria/profiles/{gone}.toml"))
            .exists()
    );
    gui.rt.block_on(daemon.stop());
}

#[test]
fn delete_works_with_no_daemon_at_all() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let gui = gui(
        root.path(),
        FakePicker::new(None),
        &root.path().join("none.sock"),
    );
    let id = id_of(&seeded, "VPS Hetzner");
    invoke(
        &gui.window,
        "profile_delete",
        json!({ "id": id.to_string() }),
    )
    .unwrap();
    assert!(
        !root
            .path()
            .join(format!("galleria/profiles/{id}.toml"))
            .exists()
    );
    let error = invoke(
        &gui.window,
        "profile_delete",
        json!({ "id": id.to_string() }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_profile_not_found");
}

// ---- no path from the webview ----------------------------------------------------------------------------------------

/// The text between the parentheses of `pub fn <name>(` (or `pub async fn <name>(`: the commands that call the daemon are
/// async), up to the `)` that closes the list (not up to the first `) ->`: a command with no return type has none). `None`
/// when the text has no such function, or the list never closes.
fn params_of<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let heads = [format!("pub fn {name}("), format!("pub async fn {name}(")];
    let (at, head) = heads
        .iter()
        .find_map(|head| source.find(head.as_str()).map(|at| (at, head)))?;
    let start = at + head.len();
    let mut depth = 1;
    for (at, c) in source[start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[start..start + at]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The parameters of a list, split at the commas that are not inside `<>`, `()` or `[]` (`HashMap<String, PathBuf>` is one).
fn split_params(params: &str) -> Vec<&str> {
    let (mut parts, mut depth, mut from) = (vec![], 0_i32, 0);
    let mut previous = ' ';
    for (at, c) in params.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' if previous == '-' => {} // the arrow of a `Fn(u8) -> u8`
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&params[from..at]);
                from = at + 1;
            }
            _ => {}
        }
        previous = c;
    }
    parts.push(&params[from..]);
    parts.retain(|p| !p.trim().is_empty());
    parts
}

/// A hand-made source: a command with no return type, followed by a line that is not a parameter of it.
const NO_RETURN_TYPE: &str = "\
#[tauri::command]
pub fn daemon_retry(state: State<'_, SupervisorHandle>) {
    state.retry_now();
}

use std::path::Path;

#[tauri::command]
pub fn other(id: String) -> Result<(), Error> {
    Ok(())
}
";

#[test]
fn params_end_at_the_parenthesis_that_closes_the_list() {
    assert_eq!(
        params_of(NO_RETURN_TYPE, "daemon_retry"),
        Some("state: State<'_, SupervisorHandle>")
    );
    assert_eq!(params_of(NO_RETURN_TYPE, "other"), Some("id: String"));
    assert_eq!(params_of(NO_RETURN_TYPE, "missing"), None);
    // a parameter that has parentheses of its own, a list that spans lines, and one with a trailing comma
    let source = "pub fn a(f: Box<dyn Fn(u8) -> u8>, g: (u8, u8),\n    h: Vec<Path>,\n) -> () {}\npub fn b() {}";
    assert_eq!(
        params_of(source, "a"),
        Some("f: Box<dyn Fn(u8) -> u8>, g: (u8, u8),\n    h: Vec<Path>,\n")
    );
    assert_eq!(params_of(source, "b"), Some(""));
    // a list that never closes is not guessed
    assert_eq!(params_of("pub fn c(a: u8", "c"), None);
}

#[test]
fn params_are_split_at_the_commas_of_the_list_only() {
    assert_eq!(
        split_params("a: HashMap<String, PathBuf>, f: Box<dyn Fn(u8, u8) -> u8>, g: (u8, u8),\n"),
        [
            "a: HashMap<String, PathBuf>",
            " f: Box<dyn Fn(u8, u8) -> u8>",
            " g: (u8, u8)"
        ]
    );
    assert!(split_params("").is_empty());
}

/// The parameters of every command of `COMMANDS`, read from the sources: no command takes a path (spec §3.2, rule 3).
#[test]
fn no_command_takes_a_path() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let build = fs::read_to_string(crate_dir.join("build.rs")).unwrap();
    let list = build
        .split("const COMMANDS: &[&str] = &[")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    let commands: Vec<&str> = list
        .split(',')
        .map(|c| c.trim().trim_matches('"'))
        .filter(|c| !c.is_empty())
        .collect();
    // each file on its own, in a fixed order: a command is looked for, and read, inside the file that has it
    let mut files: Vec<PathBuf> = fs::read_dir(crate_dir.join("src/commands"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    let sources: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(path).unwrap(),
            )
        })
        .collect();
    let mut checked = 0;
    for command in &commands {
        let found: Vec<(&str, &str)> = sources
            .iter()
            .filter_map(|(file, source)| Some((file.as_str(), params_of(source, command)?)))
            .collect();
        assert_eq!(
            found.len(),
            1,
            "`pub fn {command}(` must be in exactly one file of src/commands: {found:?}"
        );
        for param in split_params(found[0].1) {
            let Some((name, ty)) = param.split_once(':') else {
                continue;
            };
            let (name, ty) = (name.trim(), ty.trim());
            assert!(
                !["path", "file", "filename", "dir", "folder", "directory"].contains(&name),
                "{command} takes `{name}`"
            );
            assert!(
                !ty.contains("Path") && !ty.contains("OsStr"),
                "{command}: `{name}` is a `{ty}`"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, commands.len());
    assert!(commands.contains(&"profile_export"), "the list was read");
    // the six of B6 are in the list that was read, and so were looked at one by one (plan B6, T7): none takes a path, a file or a folder
    for command in [
        "tray_labels",
        "window_show",
        "unsaved_set",
        "window_hide",
        "app_quit",
        "update_view",
    ] {
        assert!(commands.contains(&command), "{command} is not in COMMANDS");
    }
}
