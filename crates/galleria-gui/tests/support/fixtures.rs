//! The files an import is tried on (plan B4, T1): written into a folder, so that a test, the smoke test and the screen
//! reader check import exactly the same bytes. Shared by `tests/import.rs`, `crates/galleria-profile/tests/import_file.rs`
//! and `examples/seed-rubrica.rs` (which include this file with `#[path]`, like `seed.rs`).
//!
//! Every secret carries `SEED-SECRET-` so that a test can search for it in whatever a command returns. The hosts are on
//! `example` domains.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::{fs, path::Path};

pub const SECRET_MARK: &str = "SEED-SECRET-";

pub const CA: &str =
    "-----BEGIN CERTIFICATE-----\nSEED-PUBLIC-CA-fixture\n-----END CERTIFICATE-----";
pub const CERT: &str =
    "-----BEGIN CERTIFICATE-----\nSEED-PUBLIC-CERT-fixture\n-----END CERTIFICATE-----";
pub const KEY: &str =
    "-----BEGIN PRIVATE KEY-----\nSEED-SECRET-fixture-key\n-----END PRIVATE KEY-----";
pub const TLS_CRYPT: &str = "-----BEGIN OpenVPN Static key V1-----\nSEED-SECRET-fixture-tls-crypt\n-----END OpenVPN Static key V1-----";

const HEAD: &str =
    "client\ndev tun\nproto udp\nremote vpn.fixture.example 1194\nnobind\nauth-user-pass\n";

/// 32 bytes of base64, like a real public key.
const PUBLIC_KEY: &str = "U0VFRC1QVUJMSUMtaG9tZWxhYi4uLi4uLi4uLi4uLi4=";

/// The `.ovpn` of the fixtures with the certificates and keys written inside it, as blocks.
pub fn inline_ovpn() -> String {
    format!(
        "{HEAD}<ca>\n{CA}\n</ca>\n<cert>\n{CERT}\n</cert>\n<key>\n{KEY}\n</key>\n<tls-crypt>\n{TLS_CRYPT}\n</tls-crypt>\n"
    )
}

/// The same profile with the certificates and keys in files next to it.
pub fn separate_ovpn() -> String {
    format!("{HEAD}ca ca.crt\ncert client.crt\nkey client.key\ntls-crypt tls-crypt.key\n")
}

/// What the profile of the laboratory (`lab/server/make-profiles.py`, «malicious») adds to a good one: the lines are the
/// same, and `fixture_matches_the_lab_malicious_profile` keeps them so.
pub const MALICIOUS_LINES: &str = "dev tun\nverb 3\nscript-security 3\nup /usr/bin/touch /tmp/PWNED-up\nplugin /tmp/PWNED-plugin\nmanagement 0.0.0.0 7505\nstatus /tmp/PWNED-status\nwritepid /tmp/PWNED-writepid\nsetenv EVIL 1\n";

pub fn malicious_ovpn() -> String {
    format!("{}{MALICIOUS_LINES}", inline_ovpn())
}

/// `up` and `down` with the two scripts that set the DNS of the server, which Galleria does on its own.
pub fn dns_ovpn() -> String {
    format!(
        "{}up /etc/openvpn/update-resolv-conf\ndown /etc/openvpn/update-systemd-resolved\n",
        inline_ovpn()
    )
}

pub fn outside_ovpn() -> String {
    format!("{HEAD}ca ../ca.crt\n")
}

/// WireGuard, with a script in `PostUp` and a shared key.
pub fn office_conf() -> String {
    format!(
        "[Interface]\nPrivateKey = {SECRET_MARK}office-private\nAddress = 10.8.0.2/32\nDNS = 10.8.0.1\nPostUp = /usr/bin/touch /tmp/PWNED-postup\n\n[Peer]\nPublicKey = {PUBLIC_KEY}\nPresharedKey = {SECRET_MARK}office-psk\nEndpoint = office.fixture.example:51820\nAllowedIPs = 0.0.0.0/0\nPersistentKeepalive = 25\n"
    )
}

/// An OpenVPN profile whose file ends in `.conf` (Q39).
pub fn server_conf() -> String {
    inline_ovpn()
}

/// Where the files of one `write_fixtures` are.
pub struct Fixtures {
    pub inline: std::path::PathBuf,
    pub separate: std::path::PathBuf,
    pub malicious: std::path::PathBuf,
    pub dns: std::path::PathBuf,
    pub outside: std::path::PathBuf,
    pub office: std::path::PathBuf,
    pub server: std::path::PathBuf,
}

/// Writes every file of the fixtures into `dir` (made if it is not there). The file that `outside.ovpn` points to is not
/// written: it is not inside the folder, and that is the point.
pub fn write_fixtures(dir: &Path) -> Fixtures {
    fs::create_dir_all(dir).unwrap();
    let put = |name: &str, text: &str| {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    };
    put("ca.crt", &format!("{CA}\n"));
    put("client.crt", &format!("{CERT}\n"));
    put("client.key", &format!("{KEY}\n"));
    put("tls-crypt.key", &format!("{TLS_CRYPT}\n"));
    Fixtures {
        inline: put("inline.ovpn", &inline_ovpn()),
        separate: put("separate.ovpn", &separate_ovpn()),
        malicious: put("malicious.ovpn", &malicious_ovpn()),
        dns: put("dns.ovpn", &dns_ovpn()),
        outside: put("outside.ovpn", &outside_ovpn()),
        office: put("office.conf", &office_conf()),
        server: put("server.conf", &server_conf()),
    }
}
