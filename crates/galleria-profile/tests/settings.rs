//! `galleria.toml` (the groups of the rubrica) and the lenient listing of the profiles (plan B3, T3, Q29, Q30).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_profile::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

fn settings_path(root: &Path) -> PathBuf {
    root.join("galleria.toml")
}

fn write_private(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn wireguard(name: &str) -> Profile {
    Profile::new(
        name,
        ProtocolConfig::Wireguard(WireGuardProfile {
            addresses: vec!["10.0.0.2/32".into()],
            dns: vec![],
            search_domains: vec![],
            mtu: None,
            peers: vec![WireGuardPeer {
                public_key: "PUBLIC".into(),
                endpoint: Some("vpn.example:51820".into()),
                allowed_ips: vec!["0.0.0.0/0".into()],
                persistent_keepalive: None,
            }],
        }),
    )
}

#[test]
fn a_fresh_store_has_no_groups() {
    let root = private_dir();
    let store = ProfileStore::open(root.path()).unwrap();
    // `open` writes `schema_version = 1` alone, and that stays valid.
    let settings = store.settings().unwrap();
    assert_eq!(settings.schema_version, 1);
    assert!(settings.groups.is_empty());
}

#[test]
fn groups_are_read_in_order() {
    let root = private_dir();
    let store = ProfileStore::open(root.path()).unwrap();
    write_private(
        &settings_path(root.path()),
        "schema_version = 1\ngroups = [\"Lavoro\", \"Personale\", \"Clienti\"]\n",
    );
    assert_eq!(
        store.settings().unwrap().groups,
        ["Lavoro", "Personale", "Clienti"]
    );
}

#[test]
fn unknown_keys_are_rejected() {
    let root = private_dir();
    let store = ProfileStore::open(root.path()).unwrap();
    write_private(
        &settings_path(root.path()),
        "schema_version = 1\ngroups = []\ncolour = \"red\"\n",
    );
    assert!(matches!(store.settings(), Err(StoreError::Invalid(_))));
}

#[test]
fn newer_schema_is_reported() {
    let root = private_dir();
    let store = ProfileStore::open(root.path()).unwrap();
    write_private(
        &settings_path(root.path()),
        "schema_version = 2\nsomething_new = true\n",
    );
    assert!(matches!(
        store.settings(),
        Err(StoreError::NewerSchema { found: 2 })
    ));
}

#[test]
fn list_lenient_skips_a_broken_profile_and_keeps_the_others() {
    let root = private_dir();
    let store = ProfileStore::open(root.path()).unwrap();
    for name in ["Uno", "Due", "Tre"] {
        store.save(&wireguard(name)).unwrap();
    }
    let profiles = root.path().join("profiles");
    let good = wireguard("Quattro");
    let newer = good
        .to_toml()
        .unwrap()
        .replace("schema_version = 1", "schema_version = 99");
    write_private(&profiles.join("newer.toml"), &newer);
    write_private(&profiles.join("broken.toml"), "this is = = not toml");

    let (found, skipped) = store.list_lenient().unwrap();
    let names: Vec<_> = found.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Due", "Tre", "Uno"]);
    let mut by_file: Vec<_> = skipped
        .iter()
        .map(|s| (s.file_name.as_str(), &s.error))
        .collect();
    by_file.sort_by_key(|(name, _)| *name);
    assert_eq!(by_file.len(), 2);
    assert_eq!(by_file[0].0, "broken.toml");
    assert!(matches!(by_file[0].1, StoreError::Invalid(_)));
    assert_eq!(by_file[1].0, "newer.toml");
    assert!(matches!(
        by_file[1].1,
        StoreError::NewerSchema { found: 99 }
    ));
    // The strict listing of the phase 1 still fails as a whole: that is why the lenient one exists.
    assert!(store.list().is_err());
}

#[test]
fn list_lenient_still_fails_on_insecure_permissions_of_the_folder() {
    let root = private_dir();
    let store = ProfileStore::open(root.path()).unwrap();
    store.save(&wireguard("Uno")).unwrap();
    let profiles = root.path().join("profiles");
    fs::set_permissions(&profiles, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        store.list_lenient(),
        Err(StoreError::InsecurePermissions { mode: 0o755, .. })
    ));
}
