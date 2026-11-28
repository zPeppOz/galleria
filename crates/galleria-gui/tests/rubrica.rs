#![cfg(unix)]
//! The rubrica in reading (plan B3, T3): the views, the two commands on the mock runtime, the errors, and that no
//! command returns a secret (R3). Everything runs without a daemon: the socket given to the app does not exist.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;

use galleria_gui::{
    configure_with,
    export::SavePicker,
    rubrica::{Rubrica, openvpn_view, summarize},
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use galleria_ipc::Protocol;
use galleria_profile::{
    OpenVpnProfile, Profile, ProfileStore, ProtocolConfig, WireGuardPeer, WireGuardProfile,
};
use rstest::rstest;
use seed::{GROUPS, SECRET_MARK, seed};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};

fn rubrica_in(root: &Path) -> (Rubrica, Arc<UiStateFile>) {
    let state = Arc::new(UiStateFile::new(root.join("state/galleria/state.toml")));
    (
        Rubrica::with_file_secrets(root.join("galleria"), Arc::clone(&state)),
        state,
    )
}

fn openvpn(config: &str) -> Profile {
    Profile::new(
        "Prova",
        ProtocolConfig::Openvpn(OpenVpnProfile {
            config: config.to_owned(),
        }),
    )
}

fn wireguard(endpoint: Option<&str>) -> Profile {
    Profile::new(
        "Prova",
        ProtocolConfig::Wireguard(WireGuardProfile {
            addresses: vec!["10.0.0.2/32".into()],
            dns: vec![],
            search_domains: vec![],
            mtu: None,
            peers: vec![WireGuardPeer {
                public_key: "PUBLIC".into(),
                endpoint: endpoint.map(str::to_owned),
                allowed_ips: vec!["0.0.0.0/0".into()],
                persistent_keepalive: None,
            }],
        }),
    )
}

#[test]
fn lists_what_the_cli_import_wrote() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());

    let view = rubrica.list().unwrap();
    let mut names: Vec<_> = view.profiles.iter().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "Cliente Rossi",
            "Datacenter Milano",
            "Homelab",
            "Ufficio Napoli",
            "VPS Hetzner"
        ]
    );
    assert_eq!(view.groups, GROUPS);
    assert!(view.skipped.is_empty());

    let by_name = |name: &str| view.profiles.iter().find(|p| p.name == name).unwrap();
    let office = by_name("Ufficio Napoli");
    assert_eq!(office.protocol, Protocol::Openvpn);
    assert_eq!(office.group.as_deref(), Some("Lavoro"));
    assert!(office.favorite);
    assert_eq!(office.server.as_deref(), Some("vpn.ufficio.example:1194"));
    assert_eq!(
        by_name("Homelab").server.as_deref(),
        Some("casa.example:51820")
    );
    assert!(!by_name("VPS Hetzner").favorite);
    assert_eq!(
        by_name("Ufficio Napoli").id,
        seeded
            .profiles
            .iter()
            .find(|(n, _)| n == "Ufficio Napoli")
            .unwrap()
            .1
    );
}

#[test]
fn last_use_comes_from_the_state_file() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let id = seeded.profiles[0].1;
    state
        .touch_last_used(id, time::OffsetDateTime::UNIX_EPOCH)
        .unwrap();
    let view = rubrica.list().unwrap();
    let used: Vec<_> = view
        .profiles
        .iter()
        .filter_map(|p| p.last_used.as_deref())
        .collect();
    assert_eq!(used, ["1970-01-01T00:00:00Z"]);
}

#[test]
fn a_rubrica_that_does_not_exist_is_empty_and_is_not_created() {
    let root = tempfile::tempdir().unwrap();
    let (rubrica, _) = rubrica_in(root.path());
    let view = rubrica.list().unwrap();
    assert!(view.profiles.is_empty() && view.groups.is_empty() && view.skipped.is_empty());
    assert!(
        !root.path().join("galleria").exists(),
        "a read must not create the store"
    );
}

#[rstest]
#[case::one_remote("client\nremote vpn.example 1194 udp\n", Some("vpn.example:1194"))]
#[case::first_of_two(
    "client\nremote a.example 1194\nremote b.example 443 tcp\n",
    Some("a.example:1194")
)]
#[case::no_port("client\nremote a.example\n", Some("a.example"))]
#[case::port_directive("client\nport 4000\nremote a.example\n", Some("a.example:4000"))]
#[case::in_a_connection_block(
    "client\n<connection>\nremote conn.example 443 tcp\n</connection>\n<connection>\nremote other.example 1194 udp\n</connection>\n",
    Some("conn.example:443")
)]
#[case::ipv6("client\nremote 2001:db8::1 1194\n", Some("[2001:db8::1]:1194"))]
#[case::no_remote("client\nnobind\n", None)]
fn server_comes_from_the_first_remote_or_endpoint(
    #[case] config: &str,
    #[case] expected: Option<&str>,
) {
    let summary = summarize(&openvpn(config), None);
    assert_eq!(summary.server.as_deref(), expected);
}

#[rstest]
#[case::endpoint(Some("vpn.example:51820"), Some("vpn.example:51820"))]
#[case::without_endpoint(None, None)]
fn wireguard_server_is_the_endpoint_of_the_first_peer(
    #[case] endpoint: Option<&str>,
    #[case] expected: Option<&str>,
) {
    assert_eq!(
        summarize(&wireguard(endpoint), None).server.as_deref(),
        expected
    );
}

#[rstest]
#[case::default_is_udp("remote a.example 1194\n", ["udp"])]
#[case::global_proto("proto tcp-client\nremote a.example 443\n", ["tcp"])]
#[case::proto_after_remote("remote a.example 443\nproto tcp\n", ["tcp"])]
#[case::remote_own("proto udp\nremote a.example 443 tcp\n", ["tcp"])]
#[case::udp6("proto udp6\nremote a.example 1194\n", ["udp"])]
fn remotes_carry_the_transport(#[case] config: &str, #[case] expected: [&str; 1]) {
    let view = openvpn_view(&openvpn(config).openvpn.unwrap().config, &[]);
    let transports: Vec<_> = view
        .remotes
        .iter()
        .map(|r| r.transport.as_deref().unwrap())
        .collect();
    assert_eq!(transports, expected);
}

#[rstest]
#[case::nothing("client\n", [false, false, false])]
#[case::user_and_password("client\nauth-user-pass\n", [false, true, false])]
#[case::with_a_code("client\nauth-user-pass\nstatic-challenge \"Code\" 1\n", [false, true, true])]
#[case::cert_block("client\n<cert>\nX\n</cert>\n", [true, false, false])]
#[case::cert_directive("client\ncert client.crt\n", [true, false, false])]
#[case::pkcs12_directive("client\npkcs12 client.p12\n", [true, false, false])]
fn auth_flags_follow_the_directives(#[case] config: &str, #[case] expected: [bool; 3]) {
    let flags = openvpn_view(config, &[]).auth;
    assert_eq!([flags.certificate, flags.user_pass, flags.otp], expected);
}

#[test]
fn a_pkcs12_kept_in_the_secrets_counts_as_a_certificate() {
    let flags = openvpn_view("client\n", &["openvpn.block.pkcs12".to_owned()]).auth;
    assert!(flags.certificate);
}

#[test]
fn the_detail_has_the_names_of_the_secrets_and_not_their_values() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = |name: &str| seeded.profiles.iter().find(|(n, _)| n == name).unwrap().1;

    let office = rubrica.get(id("Ufficio Napoli")).unwrap();
    assert_eq!(
        office.secret_refs,
        ["openvpn.block.key", "openvpn.block.tls-crypt"]
    );
    let ovpn = office.openvpn.as_ref().unwrap();
    assert_eq!(ovpn.remotes[0].host, "vpn.ufficio.example");
    assert_eq!(ovpn.remotes[0].port, Some(1194));
    assert!(ovpn.auth.certificate && !ovpn.auth.user_pass && !ovpn.auth.otp);
    assert!(office.wireguard.is_none());

    let homelab = rubrica.get(id("Homelab")).unwrap();
    assert!(homelab.openvpn.is_none());
    assert_eq!(
        homelab.wireguard.as_ref().unwrap().peers[0]
            .endpoint
            .as_deref(),
        Some("casa.example:51820")
    );
    assert_eq!(
        homelab.secret_refs,
        ["wireguard.private_key", "wireguard.peer.0.preshared_key"]
    );
    let rossi = rubrica.get(id("Cliente Rossi")).unwrap();
    let flags = &rossi.openvpn.as_ref().unwrap().auth;
    assert!(flags.user_pass && flags.otp);
}

// ---- the commands ----------------------------------------------------------------------------------------------------

fn app(
    rubrica: Rubrica,
    state: Arc<UiStateFile>,
) -> (tauri::App<MockRuntime>, WebviewWindow<MockRuntime>) {
    // `manage` keeps the first value of a type: the rubrica of the test goes in before the one of the setup.
    let mut app = configure_with(
        mock_builder().manage(Arc::new(rubrica)),
        // no daemon at all: the socket does not exist
        std::env::temp_dir().join("galleria-no-daemon-rubrica.sock"),
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
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn works_without_any_daemon() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);

    let list = invoke(&window, "profiles_list", json!({})).unwrap();
    assert_eq!(list["profiles"].as_array().unwrap().len(), 5);
    assert_eq!(list["groups"], json!(["Lavoro", "Personale", "Clienti"]));
    assert_eq!(list["skipped"], json!([]));
    let id = seeded.profiles[0].1.to_string();
    let detail = invoke(&window, "profile_get", json!({ "id": id })).unwrap();
    assert_eq!(detail["name"], "Ufficio Napoli");
    assert_eq!(detail["protocol"], "openvpn");
    assert_eq!(detail["settings"]["auto_reconnect"], true);
    assert_eq!(
        detail["openvpn"]["remotes"][0]["host"],
        "vpn.ufficio.example"
    );
}

#[test]
fn no_read_command_returns_a_secret() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);

    // The seed really keeps secrets: without this the search below would prove nothing.
    let on_disk = fs::read_dir(root.path().join("galleria/secrets"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect::<String>();
    assert!(on_disk.contains(SECRET_MARK), "the seed has no secrets");

    let mut seen = vec![
        invoke(&window, "profiles_list", json!({}))
            .unwrap()
            .to_string(),
    ];
    for (_, id) in &seeded.profiles {
        seen.push(
            invoke(&window, "profile_get", json!({ "id": id.to_string() }))
                .unwrap()
                .to_string(),
        );
    }
    assert_eq!(seen.len(), 6);
    for json in seen {
        assert!(
            !json.contains(SECRET_MARK),
            "a command returned a secret: {json}"
        );
        assert!(!json.contains("PRIVATE KEY"), "a command returned a key");
    }
}

/// A window «Save» that always picks the same place.
struct PicksAt(std::path::PathBuf);

impl SavePicker for PicksAt {
    fn pick(&self, _suggested_name: &str) -> Option<std::path::PathBuf> {
        Some(self.0.clone())
    }
}

/// R3 after a round of work (plan B3, T9): edit, raw check, groups, duplicate and export. Every answer of every command
/// of B3 is searched for the secrets of the seed, and the answers are not empty, so the search proves something.
#[test]
fn no_read_command_returns_a_secret_after_a_round_of_edits() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let target = root.path().join("exported.ovpn");
    let picker: Arc<dyn SavePicker> = Arc::new(PicksAt(target.clone()));
    let mut app = configure_with(
        mock_builder().manage(Arc::new(rubrica)).manage(picker),
        std::env::temp_dir().join("galleria-no-daemon-rubrica-round.sock"),
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

    let mut seen: Vec<(&str, String)> = vec![];
    let mut note = |what: &'static str, answer: Result<Value, Value>| {
        let value = answer.unwrap_or_else(|e| e);
        let text = value.to_string();
        assert!(text.len() > 2, "{what} answered nothing: {text}");
        seen.push((what, text));
        value
    };
    let ovpn = id_of(&seeded, "Ufficio Napoli").to_string();
    let wg = id_of(&seeded, "Homelab").to_string();
    let list = note("profiles_list", invoke(&window, "profiles_list", json!({})));
    let modified = |id: &str| {
        list["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .unwrap()["modified"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    note(
        "profile_get",
        invoke(&window, "profile_get", json!({ "id": ovpn })),
    );
    note(
        "profile_get (wireguard)",
        invoke(&window, "profile_get", json!({ "id": wg })),
    );
    let updated = note(
        "profile_update",
        invoke(
            &window,
            "profile_update",
            json!({ "id": ovpn, "expectedModified": modified(&ovpn), "patch": { "name": "Sede", "favorite": false } }),
        ),
    );
    note(
        "profile_validate_raw",
        invoke(
            &window,
            "profile_validate_raw",
            json!({ "id": ovpn, "text": "client\nup /tmp/x.sh\n" }),
        ),
    );
    note(
        "profile_update (refused raw text)",
        invoke(
            &window,
            "profile_update",
            json!({ "id": ovpn, "expectedModified": updated["modified"], "patch": { "openvpn_config": "client\nup /tmp/x.sh\n" } }),
        ),
    );
    note(
        "group_create",
        invoke(&window, "group_create", json!({ "name": "Nuovo" })),
    );
    note(
        "group_rename",
        invoke(
            &window,
            "group_rename",
            json!({ "from": "Nuovo", "to": "Famiglia" }),
        ),
    );
    let copy = note(
        "profile_duplicate",
        invoke(
            &window,
            "profile_duplicate",
            json!({ "id": wg, "name": "Homelab (copy)" }),
        ),
    );
    note(
        "profile_get (the copy)",
        invoke(&window, "profile_get", json!({ "id": copy["id"] })),
    );
    note(
        "profile_export",
        invoke(&window, "profile_export", json!({ "id": ovpn })),
    );
    note(
        "profile_delete",
        invoke(&window, "profile_delete", json!({ "id": copy["id"] })),
    );
    note(
        "profiles_list (after)",
        invoke(&window, "profiles_list", json!({})),
    );
    // the export is where the keys go, and the only place: the window got `{ saved }` and nothing more
    assert!(
        fs::read_to_string(&target).unwrap().contains(SECRET_MARK),
        "the export has the keys"
    );
    for (what, text) in &seen {
        assert!(
            !text.contains(SECRET_MARK) && !text.contains("PRIVATE KEY"),
            "{what} returned a secret: {text}"
        );
        assert!(
            !text.contains(root.path().to_str().unwrap()),
            "{what} returned a path of the machine: {text}"
        );
    }
    assert_eq!(seen.len(), 13);
}

#[test]
fn profile_get_refuses_what_is_not_an_id_and_says_when_there_is_none() {
    let root = tempfile::tempdir().unwrap();
    seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);
    // a path, a name or a traversal never reaches the disk: the argument is a UUID or nothing
    for bad in ["../../etc/passwd", "Homelab", ""] {
        assert!(
            invoke(&window, "profile_get", json!({ "id": bad })).is_err(),
            "{bad}"
        );
    }
    let err = invoke(
        &window,
        "profile_get",
        json!({ "id": "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11" }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "gui_profile_not_found");
}

#[test]
fn a_broken_profile_is_listed_apart_and_the_others_stay() {
    let root = tempfile::tempdir().unwrap();
    let galleria = root.path().join("galleria");
    let seeded = seed(&galleria, false);
    let profiles = galleria.join("profiles");
    let newer = fs::read_to_string(profiles.join(format!("{}.toml", seeded.profiles[0].1)))
        .unwrap()
        .replace("schema_version = 1", "schema_version = 99");
    for (file, text) in [
        ("from-the-future.toml", newer.as_str()),
        ("broken.toml", "= ="),
    ] {
        fs::write(profiles.join(file), text).unwrap();
        fs::set_permissions(profiles.join(file), fs::Permissions::from_mode(0o600)).unwrap();
    }
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);
    let list = invoke(&window, "profiles_list", json!({})).unwrap();
    assert_eq!(list["profiles"].as_array().unwrap().len(), 5);
    assert_eq!(
        list["skipped"],
        json!([
            { "file_name": "broken.toml", "code": "gui_profile_invalid" },
            { "file_name": "from-the-future.toml", "code": "gui_profile_newer" }
        ])
    );
}

#[test]
fn store_errors_map_to_gui_codes() {
    let root = tempfile::tempdir().unwrap();
    let galleria = root.path().join("galleria");
    seed(&galleria, false);
    fs::set_permissions(&galleria, fs::Permissions::from_mode(0o755)).unwrap();
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);

    let err = invoke(&window, "profiles_list", json!({})).unwrap_err();
    assert_eq!(err["code"], "gui_store_insecure");
    assert_eq!(err["data"]["path"], galleria.to_str().unwrap());
    assert_eq!(err["data"]["mode"], "755");
    assert!(
        err["data"]["fix"]
            .as_str()
            .unwrap()
            .starts_with("chmod 700 '"),
        "{err}"
    );
    assert!(err["message"].as_str().unwrap().contains("chmod"));
    // the same for the detail
    let err = invoke(
        &window,
        "profile_get",
        json!({ "id": "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11" }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "gui_store_insecure");

    // a file where the store should be: not readable, a different code
    fs::set_permissions(&galleria, fs::Permissions::from_mode(0o700)).unwrap();
    let blocked = tempfile::tempdir().unwrap();
    fs::write(blocked.path().join("galleria"), "x").unwrap();
    let (rubrica, state) = rubrica_in(blocked.path());
    let (_app2, window2) = app(rubrica, state);
    let err = invoke(&window2, "profiles_list", json!({})).unwrap_err();
    assert_eq!(err["code"], "gui_store_unreadable");
}

#[test]
fn the_store_of_a_test_is_the_one_in_the_tempdir_and_never_the_home() {
    // The default folder is only used when the setup builds its own Rubrica; the tests above manage theirs first.
    let root = tempfile::tempdir().unwrap();
    let (rubrica, _) = rubrica_in(root.path());
    ProfileStore::open(&root.path().join("galleria")).unwrap();
    assert!(rubrica.list().unwrap().profiles.is_empty());
}

// ---- editing (plan B3, T5) -------------------------------------------------------------------------------------------

use galleria_gui::rubrica::{FieldIssue, ProfilePatch, RubricaError, check_wireguard};
use galleria_profile::{SecretStore, to_connect_params};
use uuid::Uuid;

fn patch(value: Value) -> ProfilePatch {
    serde_json::from_value(value).unwrap()
}

fn id_of(seeded: &seed::Seeded, name: &str) -> Uuid {
    seeded.profiles.iter().find(|(n, _)| n == name).unwrap().1
}

fn file_of(root: &Path, id: Uuid) -> std::path::PathBuf {
    root.join(format!("galleria/profiles/{id}.toml"))
}

fn fields(error: RubricaError) -> Vec<(String, &'static str)> {
    match error {
        RubricaError::Invalid(issues) => issues.into_iter().map(|i| (i.field, i.code)).collect(),
        other => panic!("expected invalid fields, got {other:?}"),
    }
}

#[test]
fn update_changes_only_the_patched_fields_and_bumps_modified() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Homelab");
    let before = rubrica.get(id).unwrap();

    let after = rubrica
        .update(
            id,
            &before.summary.modified,
            patch(json!({ "name": "  Casa  ", "group": null, "favorite": false, "notes": "dietro il router" })),
        )
        .unwrap();
    assert_eq!(after.summary.name, "Casa", "the name is trimmed");
    assert_eq!(after.summary.group, None);
    assert!(!after.summary.favorite);
    assert_eq!(after.notes, "dietro il router");
    assert_ne!(after.summary.modified, before.summary.modified);
    // what the patch did not name is as it was
    assert_eq!(after.created, before.created);
    assert_eq!(after.tags, before.tags);
    assert_eq!(after.secret_refs, before.secret_refs);
    assert_eq!(
        after.settings.auto_reconnect,
        before.settings.auto_reconnect
    );
    assert_eq!(
        serde_json::to_value(&after.wireguard).unwrap(),
        serde_json::to_value(&before.wireguard).unwrap()
    );
    // and it is on the disk
    assert_eq!(rubrica.get(id).unwrap().summary.name, "Casa");

    // a group goes on with Some(Some(name)), and a patch with only the settings leaves the rest alone
    let again = rubrica
        .update(
            id,
            &after.summary.modified,
            patch(json!({ "group": "Clienti", "settings": { "ipv6_block": false, "auto_reconnect": false, "ignore_pushed_routes": true } })),
        )
        .unwrap();
    assert_eq!(again.summary.group.as_deref(), Some("Clienti"));
    assert_eq!(again.summary.name, "Casa");
    assert!(
        !again.settings.ipv6_block
            && !again.settings.auto_reconnect
            && again.settings.ignore_pushed_routes
    );
}

#[test]
fn a_stale_expected_modified_is_refused_and_nothing_is_written() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Homelab");
    let file = file_of(root.path(), id);
    let bytes = fs::read(&file).unwrap();

    let error = rubrica
        .update(
            id,
            "2000-01-01T00:00:00Z",
            patch(json!({ "name": "Altro" })),
        )
        .unwrap_err();
    assert!(matches!(error, RubricaError::Changed), "{error:?}");
    assert_eq!(fs::read(&file).unwrap(), bytes);
}

#[test]
fn openvpn_profile_refuses_a_wireguard_patch_and_the_other_way_round() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let office = rubrica.get(id_of(&seeded, "Ufficio Napoli")).unwrap();
    let homelab = rubrica.get(id_of(&seeded, "Homelab")).unwrap();

    let wireguard = serde_json::to_value(homelab.wireguard.as_ref().unwrap()).unwrap();
    let error = rubrica
        .update(
            office.summary.id,
            &office.summary.modified,
            patch(json!({ "wireguard": wireguard })),
        )
        .unwrap_err();
    assert_eq!(fields(error), [("wireguard".to_owned(), "wrong_protocol")]);

    let error = rubrica
        .update(
            homelab.summary.id,
            &homelab.summary.modified,
            patch(json!({ "openvpn_config": "client\n" })),
        )
        .unwrap_err();
    assert_eq!(
        fields(error),
        [("openvpn_config".to_owned(), "wrong_protocol")]
    );
    // the text of an OpenVPN profile is edited in the editor of T7: `tests/raw.rs`
}

#[test]
fn peer_count_cannot_change() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let homelab = rubrica.get(id_of(&seeded, "Homelab")).unwrap();
    let mut wireguard = serde_json::to_value(homelab.wireguard.as_ref().unwrap()).unwrap();
    let peer = wireguard["peers"][0].clone();
    wireguard["peers"] = json!([peer.clone(), peer]);
    let error = rubrica
        .update(
            homelab.summary.id,
            &homelab.summary.modified,
            patch(json!({ "wireguard": wireguard })),
        )
        .unwrap_err();
    assert_eq!(fields(error), [("peers".to_owned(), "count_changed")]);
}

/// A WireGuard profile the daemon accepts: 32 bytes of base64 for the key, everything else in the usual forms.
const KEY: &str = "c2VlZC1wdWJsaWMta2V5LWZvci10aGUtdGVzdHMtMDA=";

fn good_wireguard() -> WireGuardProfile {
    WireGuardProfile {
        addresses: vec!["10.8.0.2/32".into(), "fd00::2/128".into()],
        dns: vec!["10.8.0.1".into(), "fd00::1".into()],
        search_domains: vec!["corp.example".into(), "~lab.example".into()],
        mtu: Some(1380),
        peers: vec![WireGuardPeer {
            public_key: KEY.into(),
            endpoint: Some("vpn.example:51820".into()),
            allowed_ips: vec!["0.0.0.0/0".into(), "::/0".into()],
            persistent_keepalive: Some(25),
        }],
    }
}

/// The secrets of a test profile: the private key only, valid.
struct OnlyAKey;
impl SecretStore for OnlyAKey {
    fn get(
        &self,
        _: &Uuid,
        key: &str,
    ) -> Result<Option<galleria_ipc::Secret>, galleria_profile::StoreError> {
        Ok((key == "wireguard.private_key").then(|| galleria_ipc::Secret::new(KEY)))
    }
    fn set(
        &self,
        _: &Uuid,
        _: &str,
        _: &galleria_ipc::Secret,
    ) -> Result<(), galleria_profile::StoreError> {
        Ok(())
    }
    fn delete(&self, _: &Uuid, _: &str) -> Result<(), galleria_profile::StoreError> {
        Ok(())
    }
    fn delete_all(&self, _: &Uuid) -> Result<(), galleria_profile::StoreError> {
        Ok(())
    }
}

/// What the daemon says about a profile with these WireGuard settings: `None` if it accepts it, else the field it names.
fn daemon_verdict(wireguard: &WireGuardProfile) -> Option<String> {
    let mut profile = Profile::new("Prova", ProtocolConfig::Wireguard(wireguard.clone()));
    profile.secret_refs = vec!["wireguard.private_key".into()];
    let params = to_connect_params(&profile, &OnlyAKey, false).unwrap();
    let galleria_ipc::BackendParams::Wireguard(params) = params.backend else {
        panic!("not a WireGuard profile")
    };
    match galleria_backend_wireguard::validate(&params) {
        Ok(_) => None,
        Err(e) => Some(e.data.unwrap()["field"].as_str().unwrap().to_owned()),
    }
}

fn with(change: impl FnOnce(&mut WireGuardProfile)) -> WireGuardProfile {
    let mut p = good_wireguard();
    change(&mut p);
    p
}

#[rstest]
#[case::as_it_is(good_wireguard())]
#[case::no_endpoint(with(|p| p.peers[0].endpoint = None))]
#[case::ip_endpoint(with(|p| p.peers[0].endpoint = Some("203.0.113.24:51820".into())))]
#[case::ipv6_endpoint(with(|p| p.peers[0].endpoint = Some("[2001:db8::1]:51820".into())))]
#[case::no_dns_no_domains_no_mtu(with(|p| { p.dns.clear(); p.search_domains.clear(); p.mtu = None; }))]
#[case::mtu_at_the_edges(with(|p| p.mtu = Some(576)))]
#[case::mtu_at_the_top(with(|p| p.mtu = Some(9000)))]
#[case::no_keepalive(with(|p| p.peers[0].persistent_keepalive = None))]
#[case::route_all_domain(with(|p| p.search_domains = vec!["~.".into()]))]
fn what_the_gui_accepts_the_daemon_accepts(#[case] wireguard: WireGuardProfile) {
    assert_eq!(check_wireguard(&wireguard), vec![]);
    assert_eq!(daemon_verdict(&wireguard), None);
}

#[rstest]
#[case::address_without_prefix(with(|p| p.addresses = vec!["10.8.0.2".into()]), "addresses")]
#[case::address_nonsense(with(|p| p.addresses = vec!["not an address".into()]), "addresses")]
#[case::dns_is_a_name(with(|p| p.dns = vec!["dns.example".into()]), "dns")]
#[case::domain_with_a_space(with(|p| p.search_domains = vec!["bad domain".into()]), "search_domains")]
#[case::domain_with_an_empty_label(with(|p| p.search_domains = vec!["a..b".into()]), "search_domains")]
#[case::mtu_too_low(with(|p| p.mtu = Some(575)), "mtu")]
#[case::mtu_too_high(with(|p| p.mtu = Some(9001)), "mtu")]
#[case::no_peer(with(|p| p.peers.clear()), "peers")]
#[case::key_too_short(with(|p| p.peers[0].public_key = "c2hvcnQ=".into()), "peers.0.public_key")]
#[case::key_not_base64(with(|p| p.peers[0].public_key = "!!!".into()), "peers.0.public_key")]
#[case::key_without_padding(with(|p| p.peers[0].public_key = KEY.trim_end_matches('=').into()), "peers.0.public_key")]
#[case::endpoint_without_port(with(|p| p.peers[0].endpoint = Some("vpn.example".into())), "peers.0.endpoint")]
#[case::endpoint_port_zero(with(|p| p.peers[0].endpoint = Some("vpn.example:0".into())), "peers.0.endpoint")]
#[case::endpoint_port_too_big(with(|p| p.peers[0].endpoint = Some("vpn.example:70000".into())), "peers.0.endpoint")]
#[case::endpoint_with_a_bad_label(with(|p| p.peers[0].endpoint = Some("-a.example:51820".into())), "peers.0.endpoint")]
#[case::route_not_a_network(with(|p| p.peers[0].allowed_ips = vec!["0.0.0.0".into()]), "peers.0.allowed_ips")]
#[case::keepalive_too_big(with(|p| p.peers[0].persistent_keepalive = Some(65536)), "peers.0.persistent_keepalive")]
fn what_the_daemon_refuses_the_gui_refuses(
    #[case] wireguard: WireGuardProfile,
    #[case] field: &str,
) {
    assert_eq!(
        daemon_verdict(&wireguard).as_deref(),
        Some(field),
        "the daemon"
    );
    let issues = check_wireguard(&wireguard);
    assert!(
        issues.iter().any(|i| i.field == field),
        "the window accepted what the daemon refuses ({field}): {issues:?}"
    );
}

#[test]
fn the_window_is_stricter_where_the_daemon_is_not() {
    // The daemon takes a tunnel with no address and a peer that routes nothing; the window asks for one of each (D26).
    assert!(daemon_verdict(&with(|p| p.addresses.clear())).is_none());
    assert!(daemon_verdict(&with(|p| p.peers[0].allowed_ips.clear())).is_none());
    assert_eq!(
        check_wireguard(&with(|p| p.addresses.clear())),
        [FieldIssue {
            field: "addresses".into(),
            code: "required"
        }]
    );
    assert_eq!(
        check_wireguard(&with(|p| p.peers[0].allowed_ips.clear())),
        [FieldIssue {
            field: "peers.0.allowed_ips".into(),
            code: "required"
        }]
    );
}

#[test]
fn invalid_fields_are_all_reported_with_their_path() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let homelab = rubrica.get(id_of(&seeded, "Homelab")).unwrap();
    let mut wireguard = serde_json::to_value(homelab.wireguard.as_ref().unwrap()).unwrap();
    wireguard["mtu"] = json!(100);
    wireguard["peers"][0]["endpoint"] = json!("no-port");
    wireguard["peers"][0]["public_key"] = json!("short");
    let error = rubrica
        .update(
            homelab.summary.id,
            &homelab.summary.modified,
            patch(json!({ "name": "", "wireguard": wireguard })),
        )
        .unwrap_err();
    assert_eq!(
        fields(error),
        [
            ("name".to_owned(), "required"),
            ("mtu".to_owned(), "out_of_range"),
            ("peers.0.public_key".to_owned(), "invalid_key"),
            ("peers.0.endpoint".to_owned(), "invalid_endpoint"),
        ]
    );
}

#[rstest]
#[case::empty("", "required")]
#[case::only_spaces("   ", "required")]
#[case::too_long(&"a".repeat(65), "too_long")]
#[case::a_newline("one\ntwo", "invalid_chars")]
#[case::a_control_character("tab\u{7}bell", "invalid_chars")]
fn empty_name_and_overlong_name_are_refused(#[case] name: &str, #[case] code: &'static str) {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Homelab");
    let modified = rubrica.get(id).unwrap().summary.modified;
    let error = rubrica
        .update(id, &modified, patch(json!({ "name": name })))
        .unwrap_err();
    assert_eq!(fields(error), [("name".to_owned(), code)]);
    // the longest name there is room for, and a group of 48
    let long = "a".repeat(64);
    let ok = rubrica
        .update(
            id,
            &modified,
            patch(json!({ "name": long, "group": "g".repeat(48) })),
        )
        .unwrap();
    assert_eq!(ok.summary.name.len(), 64);
    let error = rubrica
        .update(
            id,
            &ok.summary.modified,
            patch(json!({ "group": "g".repeat(49) })),
        )
        .unwrap_err();
    assert_eq!(fields(error), [("group".to_owned(), "too_long")]);
}

#[test]
fn update_never_touches_the_secrets() {
    let root = tempfile::tempdir().unwrap();
    let galleria = root.path().join("galleria");
    let seeded = seed(&galleria, false);
    let (rubrica, _) = rubrica_in(root.path());
    let secrets = |id: Uuid| galleria.join(format!("secrets/{id}.toml"));
    let ids = [id_of(&seeded, "Homelab"), id_of(&seeded, "Ufficio Napoli")];
    let before: Vec<_> = ids
        .iter()
        .map(|id| fs::read(secrets(*id)).unwrap())
        .collect();

    for id in ids {
        let modified = rubrica.get(id).unwrap().summary.modified;
        rubrica
            .update(
                id,
                &modified,
                patch(json!({ "name": "Nuovo", "favorite": true })),
            )
            .unwrap();
    }
    let after: Vec<_> = ids
        .iter()
        .map(|id| fs::read(secrets(*id)).unwrap())
        .collect();
    assert_eq!(before, after);
    // and no file of secrets was added or taken away: the five that the seed wrote
    assert_eq!(fs::read_dir(galleria.join("secrets")).unwrap().count(), 5);
}

#[test]
fn a_profile_saved_by_the_gui_keeps_the_phase_1_format() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let id = id_of(&seeded, "Homelab");
    let file = file_of(root.path(), id);
    let original = Profile::from_toml(&fs::read_to_string(&file).unwrap()).unwrap();

    let mut wireguard = serde_json::to_value(original.wireguard.as_ref().unwrap()).unwrap();
    wireguard["dns"] = json!(["10.0.0.53"]);
    let saved = rubrica
        .update(
            id,
            &original.modified,
            patch(json!({ "name": "Casa", "notes": "n", "wireguard": wireguard })),
        )
        .unwrap();

    // the CLI reads it with the code of the phase 1, and what the patch did not name is equal to the letter
    let text = fs::read_to_string(&file).unwrap();
    let read = Profile::from_toml(&text).unwrap();
    assert_eq!(read.modified, saved.summary.modified);
    let mut expected = original.clone();
    expected.name = "Casa".into();
    expected.notes = "n".into();
    expected.modified = read.modified.clone();
    expected.wireguard.as_mut().unwrap().dns = vec!["10.0.0.53".into()];
    assert_eq!(read.to_toml().unwrap(), expected.to_toml().unwrap());
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn profile_update_is_a_command_and_says_why_it_refuses() {
    let root = tempfile::tempdir().unwrap();
    let seeded = seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);
    let id = id_of(&seeded, "Homelab").to_string();
    let list = invoke(&window, "profiles_list", json!({})).unwrap();
    let modified = list["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id.as_str())
        .unwrap()["modified"]
        .as_str()
        .unwrap()
        .to_owned();

    // a refusal for the fields names them
    let error = invoke(
        &window,
        "profile_update",
        json!({ "id": id, "expectedModified": modified, "patch": { "name": "" } }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_invalid_field");
    assert_eq!(
        error["data"]["fields"],
        json!([{ "field": "name", "code": "required" }])
    );

    // a patch with a key it does not know is refused whole
    assert!(
        invoke(
            &window,
            "profile_update",
            json!({ "id": id, "expectedModified": modified, "patch": { "path": "/etc/passwd" } }),
        )
        .is_err()
    );

    // it works, answers with the detail and no secret in it
    let detail = invoke(
        &window,
        "profile_update",
        json!({ "id": id, "expectedModified": modified, "patch": { "name": "Casa", "favorite": false } }),
    )
    .unwrap();
    assert_eq!(detail["name"], "Casa");
    assert_eq!(detail["favorite"], false);
    assert!(!detail.to_string().contains(SECRET_MARK));

    // the same version again is stale now
    let error = invoke(
        &window,
        "profile_update",
        json!({ "id": id, "expectedModified": modified, "patch": { "name": "Altro" } }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_profile_changed");
    let error = invoke(
        &window,
        "profile_update",
        json!({ "id": "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11", "expectedModified": modified, "patch": {} }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_profile_not_found");
}
