//! The rubrica in reading (plan B3, T3): the views, the two commands on the mock runtime, the errors, and that no
//! command returns a secret (R3). Everything runs without a daemon: the socket given to the app does not exist.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;

use galleria_gui::{
    configure_with,
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
        Rubrica::new(root.join("galleria"), Arc::clone(&state)),
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
