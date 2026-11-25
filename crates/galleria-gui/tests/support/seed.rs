//! A made-up rubrica, written with the import functions of the phase 1 (the same ones the CLI uses), so that the tests
//! of the GUI read what the CLI would have written. Shared by `tests/rubrica.rs` and `examples/seed-rubrica.rs`
//! (which includes this file with `#[path]`): the smoke test and the screen reader check start from it too.
//!
//! The five names are those of the mockup of B2. Every secret carries `SEED-SECRET-` so that a test can search for it
//! in whatever a command returns (R3). Hosts are on `example` domains.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use galleria_profile::{
    FileSecretStore, ProfileStore, SecretStore, import_openvpn, import_wireguard,
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use uuid::Uuid;

pub const SECRET_MARK: &str = "SEED-SECRET-";
pub const GROUPS: [&str; 3] = ["Lavoro", "Personale", "Clienti"];

/// What was written, in the order of the table below.
pub struct Seeded {
    pub profiles: Vec<(String, Uuid)>,
}

fn openvpn_text(host: &str, port: u16, proto: &str, tag: &str, extra: &str) -> String {
    format!(
        "client\ndev tun\nproto {proto}\nremote {host} {port}\nnobind\n{extra}\n<ca>\n-----BEGIN CERTIFICATE-----\nSEED-PUBLIC-CA-{tag}\n-----END CERTIFICATE-----\n</ca>\n<cert>\n-----BEGIN CERTIFICATE-----\nSEED-PUBLIC-CERT-{tag}\n-----END CERTIFICATE-----\n</cert>\n<key>\n-----BEGIN PRIVATE KEY-----\n{SECRET_MARK}{tag}-key\n-----END PRIVATE KEY-----\n</key>\n<tls-crypt>\n-----BEGIN OpenVPN Static key V1-----\n{SECRET_MARK}{tag}-tls-crypt\n-----END OpenVPN Static key V1-----\n</tls-crypt>\n"
    )
}

/// 32 bytes of base64, like a real public key (the window checks them): the text is `SEED-PUBLIC-<tag>` padded with dots.
fn public_key(tag: &str) -> &'static str {
    match tag {
        "homelab" => "U0VFRC1QVUJMSUMtaG9tZWxhYi4uLi4uLi4uLi4uLi4=",
        _ => "U0VFRC1QVUJMSUMtdnBzLi4uLi4uLi4uLi4uLi4uLi4=",
    }
}

fn wireguard_text(tag: &str, endpoint: &str, address: &str, preshared: bool) -> String {
    let key = public_key(tag);
    let psk = if preshared {
        format!("PresharedKey = {SECRET_MARK}{tag}-psk\n")
    } else {
        String::new()
    };
    format!(
        "[Interface]\nPrivateKey = {SECRET_MARK}{tag}-private\nAddress = {address}\nDNS = 10.0.0.1\n\n[Peer]\nPublicKey = {key}\n{psk}Endpoint = {endpoint}\nAllowedIPs = 0.0.0.0/0\nPersistentKeepalive = 25\n"
    )
}

/// Writes the rubrica into `dir` (the `galleria` folder of `XDG_CONFIG_HOME`). With `empty` only the store is created.
pub fn seed(dir: &Path, empty: bool) -> Seeded {
    let store = ProfileStore::open(dir).unwrap();
    let secrets = FileSecretStore::open(dir).unwrap();
    let mut seeded = Seeded { profiles: vec![] };
    if empty {
        return seeded;
    }
    let no_files =
        |_: &str| -> std::io::Result<String> { Err(std::io::ErrorKind::NotFound.into()) };
    // (name, group, favourite, imported)
    let table = [
        (
            "Ufficio Napoli",
            "Lavoro",
            true,
            import_openvpn(
                &openvpn_text("vpn.ufficio.example", 1194, "udp", "ufficio", ""),
                "Ufficio Napoli",
                &no_files,
            )
            .unwrap(),
        ),
        (
            "Datacenter Milano",
            "Lavoro",
            false,
            import_openvpn(
                &openvpn_text(
                    "gw.dc-mi.example",
                    443,
                    "tcp",
                    "datacenter",
                    "auth-user-pass",
                ),
                "Datacenter Milano",
                &no_files,
            )
            .unwrap(),
        ),
        (
            "Homelab",
            "Personale",
            true,
            import_wireguard(
                &wireguard_text("homelab", "casa.example:51820", "10.8.0.2/32", true),
                "Homelab",
            )
            .unwrap(),
        ),
        (
            "VPS Hetzner",
            "Personale",
            false,
            import_wireguard(
                &wireguard_text("vps", "203.0.113.24:51820", "10.9.0.2/32", false),
                "VPS Hetzner",
            )
            .unwrap(),
        ),
        (
            "Cliente Rossi",
            "Clienti",
            false,
            import_openvpn(
                &openvpn_text(
                    "vpn.rossi.example",
                    1194,
                    "udp",
                    "rossi",
                    "auth-user-pass\nstatic-challenge \"Codice\" 1",
                ),
                "Cliente Rossi",
                &no_files,
            )
            .unwrap(),
        ),
    ];
    for (name, group, favorite, imported) in table {
        let mut profile = imported.profile;
        profile.group = Some(group.to_owned());
        profile.favorite = favorite;
        store.save(&profile).unwrap();
        for (key, secret) in &imported.secrets {
            secrets.set(&profile.id, key, secret).unwrap();
        }
        seeded.profiles.push((name.to_owned(), profile.id));
    }
    let settings = dir.join("galleria.toml");
    let groups = GROUPS.map(|g| format!("\"{g}\"")).join(", ");
    fs::write(
        &settings,
        format!("schema_version = 1\ngroups = [{groups}]\n"),
    )
    .unwrap();
    fs::set_permissions(&settings, fs::Permissions::from_mode(0o600)).unwrap();
    seeded
}
