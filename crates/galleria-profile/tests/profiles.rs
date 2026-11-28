#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::{BackendParams, Secret};
use galleria_profile::*;
use rstest::rstest;
use std::{
    fs,
    os::unix::{
        fs::{PermissionsExt, symlink},
        prelude::MetadataExt,
    },
};
const EXAMPLE: &str = r#"
schema_version = 1
id = "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11"
name = "Ufficio Napoli"
protocol = "openvpn"
group = "Lavoro"
tags = []
favorite = false
notes = ""
created = "2026-11-10T10:00:00Z"
modified = "2026-11-10T10:00:00Z"
secret_refs = ["openvpn.block.key", "openvpn.block.tls-auth"]
[settings]
ipv6_block = true
auto_reconnect = true
ignore_pushed_routes = false
[openvpn]
config = "client\nremote vpn.esempio-ufficio.it 1194\n"
"#;
fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}
fn profile() -> Profile {
    Profile::from_toml(EXAMPLE).unwrap()
}
fn mode(path: &std::path::Path) -> u32 {
    fs::metadata(path).unwrap().mode() & 0o777
}
#[test]
fn profile_toml_matches_the_spec_example() {
    let p = profile();
    let a: toml::Value = toml::from_str(EXAMPLE).unwrap();
    let b: toml::Value = toml::from_str(&p.to_toml().unwrap()).unwrap();
    assert_eq!(a, b);
}
#[test]
fn unknown_field_is_an_error() {
    assert!(
        Profile::from_toml(
            &EXAMPLE.replace("favorite = false", "favorite = false\nunknown = true")
        )
        .is_err()
    );
    assert!(
        Profile::from_toml(&EXAMPLE.replace(
            "auto_reconnect = true",
            "auto_reconnect = true\nunknown = true"
        ))
        .is_err()
    );
    assert!(Profile::from_toml(&format!("{EXAMPLE}\nunknown = true\n")).is_err());
}
#[test]
fn newer_schema_version_is_reported() {
    assert!(matches!(
        Profile::from_toml(&EXAMPLE.replace("schema_version = 1", "schema_version = 2")),
        Err(StoreError::NewerSchema { found: 2 })
    ));
}
#[test]
fn save_is_atomic_and_private() {
    let dir = private_dir();
    let store = ProfileStore::open(dir.path()).unwrap();
    let p = profile();
    store.save(&p).unwrap();
    assert_eq!(mode(&dir.path().join("profiles")), 0o700);
    assert_eq!(
        mode(&dir.path().join(format!("profiles/{}.toml", p.id))),
        0o600
    );
    assert_eq!(mode(&dir.path().join("galleria.toml")), 0o600);
    assert_eq!(
        fs::read_dir(dir.path().join("profiles")).unwrap().count(),
        1
    );
}
#[test]
fn concurrent_saves_leave_a_valid_file() {
    let dir = private_dir();
    let store = ProfileStore::open(dir.path()).unwrap();
    let p = profile();
    store.save(&p).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..2 {
            let s = &store;
            let p = &p;
            scope.spawn(move || {
                for _ in 0..100 {
                    s.save(p).unwrap();
                    let read = s.find(&p.id.to_string()).unwrap();
                    assert_eq!(read.id, p.id);
                }
            });
        }
    });
    assert_eq!(
        fs::read_dir(dir.path().join("profiles")).unwrap().count(),
        1
    );
}
#[test]
fn find_by_name_and_by_id() {
    let dir = private_dir();
    let store = ProfileStore::open(dir.path()).unwrap();
    let p = profile();
    store.save(&p).unwrap();
    assert_eq!(store.find(&p.name).unwrap().id, p.id);
    assert_eq!(store.find(&p.id.to_string()).unwrap().id, p.id);
}
#[test]
fn find_reports_ambiguous_names() {
    let dir = private_dir();
    let store = ProfileStore::open(dir.path()).unwrap();
    let p = profile();
    store.save(&p).unwrap();
    let mut q = p.clone();
    q.id = uuid::Uuid::new_v4();
    store.save(&q).unwrap();
    let Err(StoreError::Ambiguous(ids)) = store.find(&p.name) else {
        panic!()
    };
    assert_eq!(ids.len(), 2);
    assert!(ids.iter().any(|(id, _)| *id == p.id));
    assert!(ids.iter().any(|(id, _)| *id == q.id));
}
#[test]
fn name_with_path_characters_never_reaches_the_filesystem() {
    let dir = private_dir();
    let store = ProfileStore::open(dir.path()).unwrap();
    let mut p = profile();
    p.name = "../../etc/x".into();
    store.save(&p).unwrap();
    assert_eq!(store.find(&p.name).unwrap().id, p.id);
    assert!(dir.path().join(format!("profiles/{}.toml", p.id)).is_file());
    assert_eq!(
        fs::read_dir(dir.path().join("profiles")).unwrap().count(),
        1
    );
}
const WG: &str = "[Interface]\nPrivateKey = private-test-key\nAddress = 10.0.0.2/24, fd00::2/64\nDNS = 10.0.0.1, fd00::1\n[Peer]\nPublicKey = public-test-key\nPresharedKey = preshared-test-key\nEndpoint = vpn.test:51820\nAllowedIPs = 0.0.0.0/0, ::/0\nPersistentKeepalive = 25\n";
#[test]
fn import_wireguard_moves_private_key_to_secrets() {
    let i = import_wireguard(WG, "Office").unwrap();
    let t = i.profile.to_toml().unwrap();
    assert!(!t.contains("private-test-key"));
    assert!(!t.contains("preshared-test-key"));
    assert!(
        i.secrets
            .iter()
            .any(|(k, v)| k == "wireguard.private_key" && v.expose() == "private-test-key")
    );
}
#[test]
fn import_wireguard_handles_comma_lists() {
    let i = import_wireguard(WG, "Office").unwrap();
    let wg = i.profile.wireguard.unwrap();
    assert_eq!(wg.addresses, ["10.0.0.2/24", "fd00::2/64"]);
    assert_eq!(wg.dns, ["10.0.0.1", "fd00::1"]);
    assert_eq!(wg.peers[0].allowed_ips, ["0.0.0.0/0", "::/0"]);
}
#[test]
fn import_wireguard_drops_scripts_with_warning() {
    let i = import_wireguard(
        &WG.replace(
            "Address =",
            "PostUp = touch /tmp/PWNED\nTable = off\nAddress =",
        ),
        "Office",
    )
    .unwrap();
    assert!(
        i.warnings
            .iter()
            .any(|w| w.directive == "PostUp" && w.kind == WarningKind::Dangerous)
    );
    assert!(
        i.warnings
            .iter()
            .any(|w| w.directive == "Table" && w.kind == WarningKind::Unsupported)
    );
    assert!(!i.profile.to_toml().unwrap().contains("PWNED"));
}
#[test]
fn import_openvpn_inlines_referenced_files() {
    let i = import_openvpn("client\nca ca.crt\n", "Office", &|path| {
        assert_eq!(path, "ca.crt");
        Ok("CERTIFICATE".into())
    })
    .unwrap();
    assert!(
        i.profile
            .openvpn
            .unwrap()
            .config
            .contains("<ca>\nCERTIFICATE\n</ca>")
    );
}
#[test]
fn import_openvpn_reports_missing_file() {
    assert!(
        matches!(import_openvpn("ca ca.crt\n","Office",&|_| Err(std::io::ErrorKind::NotFound.into())),Err(ImportError::MissingFile(p)) if p=="ca.crt")
    );
}
// Spec §8.2: referenced files are read from the folder of the .ovpn, nowhere else.
#[rstest]
#[case("/home/alice/.ssh/id_ed25519")]
#[case("/etc/ssl/certs/ca.pem")]
#[case("../ca.crt")]
#[case("certs/../../ca.crt")]
#[case("./../ca.crt")]
#[case("..")]
fn import_openvpn_rejects_files_outside_the_profile_folder(#[case] path: &str) {
    let text = format!("client\nca \"{path}\"\n");
    let result = import_openvpn(&text, "Office", &|p| panic!("must not read {p}"));
    assert!(
        matches!(&result, Err(ImportError::OutsideFolder(p)) if p == path),
        "{path}: {:?}",
        result.err()
    );
}
#[rstest]
#[case("ca.crt")]
#[case("./ca.crt")]
#[case("certs/ca.crt")]
#[case("certs/../ca.crt")]
fn import_openvpn_reads_files_inside_the_profile_folder(#[case] path: &str) {
    let text = format!("client\nca \"{path}\"\n");
    let i = import_openvpn(&text, "Office", &|p| {
        assert_eq!(p, path);
        Ok("CERTIFICATE".into())
    })
    .unwrap();
    assert!(i.profile.openvpn.unwrap().config.contains("<ca>"));
}
fn import() -> Imported {
    import_openvpn(
        "client\n<key>\nprivate-test-key\n</key>\n<tls-auth>\ntls-test-key\n</tls-auth>\n",
        "Office",
        &|_| panic!(),
    )
    .unwrap()
}
#[test]
fn import_openvpn_moves_secret_blocks_out() {
    let i = import();
    assert!(!i.profile.to_toml().unwrap().contains("private-test-key"));
    assert_eq!(i.secrets.len(), 2);
    assert_eq!(i.secrets[0].0, "openvpn.block.key");
    assert_eq!(i.secrets[1].0, "openvpn.block.tls-auth");
}
#[test]
fn import_openvpn_drops_dangerous_with_warning() {
    let i = import_openvpn("client\nup /x\n", "Office", &|_| panic!()).unwrap();
    assert_eq!(i.warnings[0].directive, "up");
    assert_eq!(i.warnings[0].kind, WarningKind::Dangerous);
    assert!(!i.profile.openvpn.unwrap().config.contains("/x"));
}
#[test]
fn import_openvpn_maps_dev_tap_to_dev_type() {
    let i = import_openvpn("client\ndev tap0\n", "Office", &|_| panic!()).unwrap();
    assert!(
        i.profile
            .openvpn
            .unwrap()
            .config
            .contains("dev-type \"tap\"")
    );
    assert!(i.warnings.is_empty());
    let i = import_openvpn("client\ndev tun\n", "Office", &|_| panic!()).unwrap();
    assert!(!i.profile.openvpn.unwrap().config.contains("dev "));
    assert!(i.warnings.is_empty());
}
#[test]
fn import_openvpn_warns_without_tls_client() {
    let i = import_openvpn("remote vpn.test\n", "Office", &|_| panic!()).unwrap();
    assert!(
        i.warnings
            .iter()
            .any(|w| w.kind == WarningKind::NoTlsClient)
    );
}
#[test]
fn imported_openvpn_always_passes_strict() {
    for text in [
        "client\nup /x\nunknown x\ndev tun\n",
        "client\n<key>\nsecret\n</key>\n",
        "<connection>\nremote vpn.test\n</connection>\n",
    ] {
        let i = import_openvpn(text, "Office", &|_| panic!()).unwrap();
        let config = i.profile.openvpn.unwrap().config;
        galleria_ovpn::check_strict(&galleria_ovpn::parse(&config).unwrap()).unwrap();
    }
}
#[test]
fn to_connect_params_reassembles_secrets() {
    let dir = private_dir();
    let store = FileSecretStore::open(dir.path()).unwrap();
    let mut i = import();
    for (k, v) in &i.secrets {
        store.set(&i.profile.id, k, v).unwrap();
    }
    i.profile.settings.ignore_pushed_routes = true;
    let p = to_connect_params(&i.profile, &store, true).unwrap();
    assert!(p.replace);
    assert!(p.options.ignore_pushed_routes);
    assert!(p.options.reconnect.enabled);
    let BackendParams::Openvpn(p) = p.backend else {
        panic!()
    };
    assert_eq!(p.secret_blocks["key"].expose(), "private-test-key\n");
    assert!(p.password.is_none());
    assert!(p.otp.is_none());
}
#[test]
fn secrets_file_is_0600_and_removed_with_profile() {
    let dir = private_dir();
    let profiles = ProfileStore::open(dir.path()).unwrap();
    let secrets = FileSecretStore::open(dir.path()).unwrap();
    let p = profile();
    profiles.save(&p).unwrap();
    secrets
        .set(&p.id, "openvpn.block.key", &Secret::new("private-test-key"))
        .unwrap();
    let path = dir.path().join(format!("secrets/{}.toml", p.id));
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(&dir.path().join("secrets")), 0o700);
    profiles.delete(&p.id).unwrap();
    assert!(!path.exists());
}
#[rstest]
#[case(0o640)]
#[case(0o604)]
#[case(0o660)]
#[case(0o644)]
fn secrets_read_checks_permissions_every_time(#[case] bad: u32) {
    let dir = private_dir();
    let store = FileSecretStore::open(dir.path()).unwrap();
    let id = uuid::Uuid::new_v4();
    store
        .set(&id, "key", &Secret::new("private-test-key"))
        .unwrap();
    assert!(store.get(&id, "key").unwrap().is_some());
    let path = dir.path().join(format!("secrets/{id}.toml"));
    fs::set_permissions(&path, fs::Permissions::from_mode(bad)).unwrap();
    assert!(
        matches!(store.get(&id,"key"),Err(StoreError::InsecurePermissions { path:p, mode }) if p==path && mode==bad)
    );
    assert_eq!(mode(&path), bad);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        store.get(&id, "key").unwrap().unwrap().expose(),
        "private-test-key"
    );
}
#[test]
fn secrets_dir_with_group_or_other_bits_is_refused() {
    let dir = private_dir();
    let store = FileSecretStore::open(dir.path()).unwrap();
    let path = dir.path().join("secrets");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();
    assert!(
        matches!(store.get(&uuid::Uuid::new_v4(),"key"),Err(StoreError::InsecurePermissions { path:p, mode:0o750 }) if p==path)
    );
}
#[test]
fn secrets_symlink_is_refused() {
    let dir = private_dir();
    let store = FileSecretStore::open(dir.path()).unwrap();
    let id = uuid::Uuid::new_v4();
    let target = dir.path().join("target");
    fs::write(&target, "invalid TOML that must never be parsed").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&target, dir.path().join(format!("secrets/{id}.toml"))).unwrap();
    assert!(matches!(
        store.get(&id, "key"),
        Err(StoreError::Invalid(_)) | Err(StoreError::Io(_))
    ));
}

#[test]
fn binary_pkcs12_references_are_encoded_and_kept_out_of_public_config() {
    let imported = import_openvpn_bytes("client\npkcs12 client.p12\n", "Office", &|path| {
        assert_eq!(path, "client.p12");
        Ok(vec![0, 255, 16])
    })
    .unwrap();
    assert_eq!(imported.secrets.len(), 1);
    assert_eq!(imported.secrets[0].0, "openvpn.block.pkcs12");
    assert_eq!(imported.secrets[0].1.expose(), "AP8Q\n");
    let public = imported.profile.to_toml().unwrap();
    assert!(!public.contains("AP8Q"));
    assert!(!public.contains("<pkcs12>"));
}
#[test]
fn pkcs12_base64_lines_fit_the_openvpn_buffer() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let bytes = vec![255; 1024];
    let imported =
        import_openvpn_bytes("client\npkcs12 bundle\n", "Office", &|_| Ok(bytes.clone())).unwrap();
    let body = imported.secrets[0].1.expose();
    assert!(body.lines().all(|line| line.len() <= 64));
    assert_eq!(
        STANDARD.decode(body.lines().collect::<String>()).unwrap(),
        bytes
    );
}
