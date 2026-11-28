#![cfg(unix)]
//! The text «Export» writes (plan B3, T8, Q31): the native file of a profile with its keys put back in, which the import
//! reads back into the same profile and the same secrets.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::Secret;
use galleria_profile::*;
use std::{fs, os::unix::fs::PermissionsExt};

fn no_files(_: &str) -> std::io::Result<String> {
    Err(std::io::ErrorKind::NotFound.into())
}

fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

const OPENVPN: &str = "client\nproto udp\nremote vpn.example 1194\nnobind\nauth-user-pass\n<ca>\n-----BEGIN CERTIFICATE-----\nPUBLIC-CA\n-----END CERTIFICATE-----\n</ca>\n<cert>\n-----BEGIN CERTIFICATE-----\nPUBLIC-CERT\n-----END CERTIFICATE-----\n</cert>\n<key>\nSECRET-KEY-LINE-1\nSECRET-KEY-LINE-2\n</key>\n<tls-crypt>\nSECRET-TLS-CRYPT\n</tls-crypt>\n";

const WIREGUARD: &str = "[Interface]\nPrivateKey = SECRET-PRIVATE\nAddress = 10.8.0.2/32\nDNS = 10.8.0.1, corp.example\nMTU = 1380\n\n[Peer]\nPublicKey = U0VFRC1QVUJMSUMtaG9tZWxhYi4uLi4uLi4uLi4uLi4=\nPresharedKey = SECRET-PSK-0\nEndpoint = casa.example:51820\nAllowedIPs = 0.0.0.0/0, ::/0\nPersistentKeepalive = 25\n\n[Peer]\nPublicKey = U0VFRC1QVUJMSUMtdnBzLi4uLi4uLi4uLi4uLi4uLi4=\nAllowedIPs = 10.9.0.0/24\n";

/// What an import gave, kept in a store as the CLI does.
fn keep(imported: &Imported, dir: &std::path::Path) -> FileSecretStore {
    let secrets = FileSecretStore::open(dir).unwrap();
    for (key, value) in &imported.secrets {
        secrets.set(&imported.profile.id, key, value).unwrap();
    }
    secrets
}

fn sorted(secrets: &[(String, Secret)]) -> Vec<(String, String)> {
    let mut list: Vec<_> = secrets
        .iter()
        .map(|(k, v)| (k.clone(), v.expose().to_owned()))
        .collect();
    list.sort();
    list
}

#[test]
fn openvpn_export_round_trips_through_the_import() {
    let dir = private_dir();
    let imported = import_openvpn(OPENVPN, "Ufficio", &no_files).unwrap();
    assert_eq!(imported.secrets.len(), 2);
    let secrets = keep(&imported, dir.path());

    let text = export_config(&imported.profile, &secrets).unwrap();
    // the keys are back in the text, as blocks
    assert!(text.contains("<key>\nSECRET-KEY-LINE-1\nSECRET-KEY-LINE-2\n</key>\n"));
    assert!(text.contains("<tls-crypt>\nSECRET-TLS-CRYPT\n</tls-crypt>\n"));
    assert!(text.ends_with('\n'));

    let again = import_openvpn(&text, "Ufficio", &no_files).unwrap();
    assert_eq!(
        again.profile.openvpn.as_ref().unwrap().config,
        imported.profile.openvpn.as_ref().unwrap().config
    );
    assert_eq!(sorted(&again.secrets), sorted(&imported.secrets));
    let mut refs = again.profile.secret_refs.clone();
    let mut had = imported.profile.secret_refs.clone();
    refs.sort();
    had.sort();
    assert_eq!(refs, had);
    assert!(again.warnings.is_empty());
}

#[test]
fn a_config_without_a_final_newline_still_gets_its_blocks_on_their_own_lines() {
    let dir = private_dir();
    let mut imported = import_openvpn(OPENVPN, "Ufficio", &no_files).unwrap();
    let config = &mut imported.profile.openvpn.as_mut().unwrap().config;
    *config = config.trim_end().to_owned();
    let secrets = keep(&imported, dir.path());
    let text = export_config(&imported.profile, &secrets).unwrap();
    let again = import_openvpn(&text, "Ufficio", &no_files).unwrap();
    assert_eq!(sorted(&again.secrets), sorted(&imported.secrets));
}

#[test]
fn wireguard_export_round_trips_through_the_import() {
    let dir = private_dir();
    let imported = import_wireguard(WIREGUARD, "Homelab").unwrap();
    let secrets = keep(&imported, dir.path());

    let text = export_config(&imported.profile, &secrets).unwrap();
    assert!(text.contains("PrivateKey = SECRET-PRIVATE\n"));
    assert!(text.contains("PresharedKey = SECRET-PSK-0\n"));
    let again = import_wireguard(&text, "Homelab").unwrap();
    assert_eq!(
        format!("{:?}", again.profile.wireguard),
        format!("{:?}", imported.profile.wireguard)
    );
    assert_eq!(sorted(&again.secrets), sorted(&imported.secrets));
    assert!(again.warnings.is_empty());
}

#[test]
fn export_fails_when_a_referenced_secret_is_missing() {
    let dir = private_dir();
    let imported = import_openvpn(OPENVPN, "Ufficio", &no_files).unwrap();
    let secrets = FileSecretStore::open(dir.path()).unwrap();
    // only one of the two keys is in the archive
    secrets
        .set(
            &imported.profile.id,
            "openvpn.block.key",
            &imported.secrets[0].1,
        )
        .unwrap();
    assert!(matches!(
        export_config(&imported.profile, &secrets),
        Err(StoreError::Invalid(_))
    ));

    // a WireGuard profile without its private key, and one with a shared key that is named and not there
    let imported = import_wireguard(WIREGUARD, "Homelab").unwrap();
    assert!(export_config(&imported.profile, &secrets).is_err());
    let without_private = private_dir();
    let without_shared = private_dir();
    let keep_all_but = |dir: &std::path::Path, without: &str| {
        let store = FileSecretStore::open(dir).unwrap();
        for (key, value) in &imported.secrets {
            if key != without {
                store.set(&imported.profile.id, key, value).unwrap();
            }
        }
        store
    };
    let store = keep_all_but(without_shared.path(), "wireguard.peer.0.preshared_key");
    assert!(export_config(&imported.profile, &store).is_err());
    let store = keep_all_but(without_private.path(), "wireguard.private_key");
    assert!(export_config(&imported.profile, &store).is_err());
}

#[test]
fn a_profile_without_key_material_exports_as_it_is() {
    // OpenVPN with nothing in the archive (a configuration that has no key block): the text, as it is
    let dir = private_dir();
    let profile = Profile::new(
        "Bare",
        ProtocolConfig::Openvpn(OpenVpnProfile {
            config: "client\nremote a.example 1194\n".into(),
        }),
    );
    let secrets = FileSecretStore::open(dir.path()).unwrap();
    assert_eq!(
        &*export_config(&profile, &secrets).unwrap(),
        "client\nremote a.example 1194\n"
    );
}
