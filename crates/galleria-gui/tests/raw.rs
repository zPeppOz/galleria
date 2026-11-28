//! The editor of the raw configuration (plan B3, T7): the check, the save that checks again, and what never leaves
//! the disk. Everything runs without a daemon.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;

use galleria_gui::{
    configure_with,
    rubrica::{ProfilePatch, Rubrica, RubricaError},
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use galleria_ipc::BackendParams;
use galleria_profile::{FileSecretStore, ProfileStore, to_connect_params};
use seed::{SECRET_MARK, seed};
use serde_json::{Value, json};
use std::{fs, path::Path, sync::Arc, time::Duration};
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};
use uuid::Uuid;

fn rubrica_in(root: &Path) -> (Rubrica, Arc<UiStateFile>) {
    let state = Arc::new(UiStateFile::new(root.join("state/galleria/state.toml")));
    (
        Rubrica::with_file_secrets(root.join("galleria"), Arc::clone(&state)),
        state,
    )
}

fn id_of(seeded: &seed::Seeded, name: &str) -> Uuid {
    seeded.profiles.iter().find(|(n, _)| n == name).unwrap().1
}

fn file_of(root: &Path, id: Uuid) -> std::path::PathBuf {
    root.join(format!("galleria/profiles/{id}.toml"))
}

fn config_patch(text: &str) -> ProfilePatch {
    serde_json::from_value(json!({ "openvpn_config": text })).unwrap()
}

/// A configuration the editor accepts, with the comments and the blank lines a person writes.
const GOOD: &str = "# my office\nclient\n\nremote vpn.ufficio.example 1194 udp   # the gateway\nnobind\n; old line\nproto udp\n\n\n<ca>\n-----BEGIN CERTIFICATE-----\nSEED-PUBLIC-CA-ufficio\n-----END CERTIFICATE-----\n</ca>\n";

fn issues(error: RubricaError) -> Vec<(usize, String, &'static str)> {
    match error {
        RubricaError::InvalidConfig(list) => list
            .into_iter()
            .map(|i| (i.line, i.directive, i.kind))
            .collect(),
        other => panic!("expected a refused configuration, got {other:?}"),
    }
}

#[test]
fn validate_reports_and_never_writes() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Ufficio Napoli");
    let before = fs::read(file_of(root.path(), id)).unwrap();

    let check = rubrica
        .validate_raw(id, "client\nup /tmp/x.sh\nfrob\n")
        .unwrap();
    assert!(!check.can_save);
    let found: Vec<_> = check
        .issues
        .iter()
        .map(|i| (i.line, i.directive.as_str(), i.kind))
        .collect();
    assert_eq!(found, [(2, "up", "dangerous"), (3, "frob", "unsupported")]);

    let clean = rubrica.validate_raw(id, GOOD).unwrap();
    assert!(clean.can_save && clean.issues.is_empty());
    assert_eq!(
        fs::read(file_of(root.path(), id)).unwrap(),
        before,
        "a check must not write"
    );
}

#[test]
fn a_missing_client_is_a_warning_and_does_not_stop_the_save() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    let text = "remote vpn.ufficio.example 1194\n";
    let check = rubrica.validate_raw(office.summary.id, text).unwrap();
    assert!(check.can_save);
    assert_eq!(check.issues.len(), 1);
    assert_eq!(
        (check.issues[0].line, check.issues[0].kind),
        (0, "no_tls_client")
    );
    let saved = rubrica
        .update(
            office.summary.id,
            &office.summary.modified,
            config_patch(text),
        )
        .unwrap();
    assert_eq!(saved.openvpn.unwrap().config, text);
}

#[test]
fn update_refuses_a_config_with_a_blocking_issue() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    let file = file_of(root.path(), office.summary.id);
    let before = fs::read(&file).unwrap();

    let error = rubrica
        .update(
            office.summary.id,
            &office.summary.modified,
            config_patch("client\nup /tmp/x.sh\nca ca.crt\n"),
        )
        .unwrap_err();
    assert_eq!(
        issues(error),
        [
            (2, "up".to_owned(), "dangerous"),
            (3, "ca".to_owned(), "file_reference")
        ]
    );
    assert_eq!(fs::read(&file).unwrap(), before);
}

#[test]
fn update_revalidates_even_after_a_clean_validate() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    // the window says the text is clean...
    assert!(
        rubrica
            .validate_raw(office.summary.id, GOOD)
            .unwrap()
            .can_save
    );
    // ...and then sends another one: only what arrives at the save counts
    let error = rubrica
        .update(
            office.summary.id,
            &office.summary.modified,
            config_patch("client\nplugin /tmp/evil.so\n"),
        )
        .unwrap_err();
    assert_eq!(issues(error), [(2, "plugin".to_owned(), "dangerous")]);
}

#[test]
fn a_secret_typed_in_the_editor_is_not_stored_and_not_echoed() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    let typed = "client\n<key>\nTYPED-IN-THE-EDITOR-KEY\n</key>\n<tls-crypt>\nTYPED-IN-THE-EDITOR-TLS\n</tls-crypt>\n";

    let check = rubrica.validate_raw(office.summary.id, typed).unwrap();
    let shown = serde_json::to_string(&check).unwrap();
    assert!(!shown.contains("TYPED-IN-THE-EDITOR"), "echoed: {shown}");
    assert!(
        check
            .issues
            .iter()
            .all(|i| i.kind == "secret_block" && !i.directive.is_empty())
    );

    let error = rubrica
        .update(
            office.summary.id,
            &office.summary.modified,
            config_patch(typed),
        )
        .unwrap_err();
    let shown = format!("{error:?} {error}");
    assert!(!shown.contains("TYPED-IN-THE-EDITOR"), "echoed: {shown}");
    // not in the profile, and not in the archive of the secrets
    for entry in fs::read_dir(root.path().join("galleria")).unwrap() {
        let path = entry.unwrap().path();
        let files = if path.is_dir() {
            fs::read_dir(&path)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect()
        } else {
            vec![path]
        };
        for file in files {
            let text = fs::read_to_string(&file).unwrap();
            assert!(
                !text.contains("TYPED-IN-THE-EDITOR"),
                "{} has the key",
                file.display()
            );
        }
    }
}

#[test]
fn saved_raw_config_is_accepted_by_the_daemon_validator() {
    // S24: the text is saved as it was written, comments and blank lines included, and the daemon takes it.
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    let saved = rubrica
        .update(
            office.summary.id,
            &office.summary.modified,
            config_patch(GOOD),
        )
        .unwrap();
    assert_eq!(saved.openvpn.as_ref().unwrap().config, GOOD);
    assert_ne!(saved.summary.modified, office.summary.modified);

    let store = ProfileStore::open(&root.path().join("galleria")).unwrap();
    let profile = store.find(&office.summary.id.to_string()).unwrap();
    assert_eq!(profile.openvpn.as_ref().unwrap().config, GOOD);
    let secrets = FileSecretStore::open(&root.path().join("galleria")).unwrap();
    let params = to_connect_params(&profile, &secrets, false).unwrap();
    let BackendParams::Openvpn(params) = params.backend else {
        panic!("not an OpenVPN profile")
    };
    galleria_backend_openvpn::config::validate(&params)
        .expect("the daemon refuses what the editor saved");
    // the secrets of the profile are the ones it had
    assert_eq!(profile.secret_refs, office.secret_refs);
}

#[test]
fn wireguard_profile_refuses_openvpn_config() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let homelab = rubrica.get(id_of(&seeded, "Homelab")).unwrap();
    let error = rubrica
        .update(
            homelab.summary.id,
            &homelab.summary.modified,
            config_patch("client\n"),
        )
        .unwrap_err();
    match error {
        RubricaError::Invalid(fields) => {
            assert_eq!(
                fields
                    .iter()
                    .map(|f| (f.field.as_str(), f.code))
                    .collect::<Vec<_>>(),
                [("openvpn_config", "wrong_protocol")]
            );
        }
        other => panic!("expected invalid fields, got {other:?}"),
    }
    assert!(matches!(
        rubrica.validate_raw(homelab.summary.id, "client\n"),
        Err(RubricaError::Invalid(_))
    ));
}

#[test]
fn a_wireguard_profile_shows_its_text_without_any_key() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let homelab = rubrica.get(id_of(&seeded, "Homelab")).unwrap();
    let text = homelab.wireguard_config.as_deref().unwrap();
    assert!(text.contains("[Interface]") && text.contains("Endpoint = casa.example:51820"));
    assert!(!text.contains("PrivateKey") && !text.contains("PresharedKey"));
    assert!(!text.contains(SECRET_MARK));
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    assert!(office.wireguard_config.is_none());
}

#[test]
fn raw_update_leaves_everything_else_alone() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    let saved = rubrica
        .update(
            office.summary.id,
            &office.summary.modified,
            config_patch(GOOD),
        )
        .unwrap();
    assert_eq!(saved.summary.name, office.summary.name);
    assert_eq!(saved.summary.group, office.summary.group);
    assert_eq!(saved.summary.favorite, office.summary.favorite);
    assert_eq!(saved.secret_refs, office.secret_refs);
    // the remotes are read from the new text
    assert_eq!(
        saved.summary.server.as_deref(),
        Some("vpn.ufficio.example:1194")
    );
}

// ---- the commands ----------------------------------------------------------------------------------------------------

fn app(
    rubrica: Rubrica,
    state: Arc<UiStateFile>,
) -> (tauri::App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let mut app = configure_with(
        mock_builder().manage(Arc::new(rubrica)),
        std::env::temp_dir().join("galleria-no-daemon-raw.sock"),
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
    (app, window)
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn the_commands_check_and_save_the_text() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);
    let id = id_of(&seeded, "Ufficio Napoli").to_string();
    let detail = invoke(&window, "profile_get", json!({ "id": id })).unwrap();
    let modified = detail["modified"].as_str().unwrap().to_owned();

    let check = invoke(
        &window,
        "profile_validate_raw",
        json!({ "id": id, "text": "client\nup /tmp/x.sh\n" }),
    )
    .unwrap();
    assert_eq!(check["can_save"], false);
    assert_eq!(
        check["issues"],
        json!([{ "line": 2, "directive": "up", "kind": "dangerous" }])
    );

    // the refusal of a save carries the same list, under the field, for the window to show
    let error = invoke(
        &window,
        "profile_update",
        json!({ "id": id, "expectedModified": modified, "patch": { "openvpn_config": "client\nup /tmp/x.sh\n" } }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_invalid_field");
    assert_eq!(
        error["data"]["fields"],
        json!([{ "field": "openvpn_config", "code": "config_issues" }])
    );
    assert_eq!(
        error["data"]["issues"],
        json!([{ "line": 2, "directive": "up", "kind": "dangerous" }])
    );
    assert!(!error.to_string().contains("/tmp/x.sh"));

    let saved = invoke(
        &window,
        "profile_update",
        json!({ "id": id, "expectedModified": modified, "patch": { "openvpn_config": GOOD } }),
    )
    .unwrap();
    assert_eq!(saved["openvpn"]["config"], GOOD);
    assert!(!saved.to_string().contains(SECRET_MARK));

    // a check of a profile that is not there
    let error = invoke(
        &window,
        "profile_validate_raw",
        json!({ "id": "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11", "text": "client\n" }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_profile_not_found");
}
