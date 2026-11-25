//! The text of a profile as the editor shows and checks it (plan B3, T7): the phase 1 classification of the directives,
//! line by line, and the `.conf` of a WireGuard profile.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::Secret;
use galleria_profile::*;
use rstest::rstest;

fn no_files(_: &str) -> std::io::Result<String> {
    Err(std::io::ErrorKind::NotFound.into())
}

const IMPORTED: &str = "client\ndev tun\nproto udp\nremote vpn.example 1194\nnobind\n# a comment\n<ca>\n-----BEGIN CERTIFICATE-----\nPUBLIC\n-----END CERTIFICATE-----\n</ca>\n<key>\nSECRET\n</key>\n<tls-crypt>\nSECRET-TLS\n</tls-crypt>\n";

const WG: &str = "[Interface]\nPrivateKey = PRIVATE-KEY\nAddress = 10.8.0.2/32, fd00::2/128\nDNS = 10.8.0.1, example.org\nMTU = 1380\n\n[Peer]\nPublicKey = U0VFRC1QVUJMSUMtaG9tZWxhYi4uLi4uLi4uLi4uLi4=\nPresharedKey = PSK-ONE\nEndpoint = casa.example:51820\nAllowedIPs = 0.0.0.0/0, ::/0\nPersistentKeepalive = 25\n\n[Peer]\nPublicKey = U0VFRC1QVUJMSUMtdnBzLi4uLi4uLi4uLi4uLi4uLi4=\nAllowedIPs = 10.9.0.0/24\n";

#[test]
fn a_config_written_by_the_import_has_no_issue() {
    let imported = import_openvpn(IMPORTED, "P", &no_files).unwrap();
    let config = imported.profile.openvpn.unwrap().config;
    assert_eq!(check_openvpn_config(&config), vec![]);
}

#[rstest]
#[case::script("client\nup /x.sh\n", 2, "up", ConfigIssueKind::Dangerous)]
#[case::script_with_args(
    "client\nroute-up \"/bin/echo hi\"\n",
    2,
    "route-up",
    ConfigIssueKind::Dangerous
)]
#[case::owned_by_the_daemon("client\ndev tun\n", 2, "dev", ConfigIssueKind::DaemonOwned)]
#[case::management(
    "client\nmanagement 127.0.0.1 7505\n",
    2,
    "management",
    ConfigIssueKind::DaemonOwned
)]
#[case::made_up(
    "client\nfrobnicate yes\n",
    2,
    "frobnicate",
    ConfigIssueKind::Unsupported
)]
#[case::file_reference("client\nca ca.crt\n", 2, "ca", ConfigIssueKind::FileReference)]
#[case::secret_block(
    "client\n<key>\nSECRET\n</key>\n",
    2,
    "key",
    ConfigIssueKind::SecretBlock
)]
#[case::secret_tls_crypt(
    "client\nremote a 1\n<tls-crypt>\nS\n</tls-crypt>\n",
    3,
    "tls-crypt",
    ConfigIssueKind::SecretBlock
)]
#[case::unknown_block(
    "client\n<frob>\nx\n</frob>\n",
    2,
    "frob",
    ConfigIssueKind::Unsupported
)]
#[case::unclosed_quote("client\nremote \"a 1\n", 2, "", ConfigIssueKind::Syntax)]
#[case::unclosed_block("client\n<ca>\nx\n", 2, "", ConfigIssueKind::Syntax)]
#[case::auth_user_pass_with_a_file(
    "client\nauth-user-pass creds.txt\n",
    2,
    "auth-user-pass",
    ConfigIssueKind::Dangerous
)]
fn each_class_of_directive_gets_its_kind_and_line(
    #[case] text: &str,
    #[case] line: usize,
    #[case] directive: &str,
    #[case] kind: ConfigIssueKind,
) {
    let issues = check_openvpn_config(text);
    assert_eq!(
        issues,
        vec![ConfigIssue {
            line,
            directive: directive.to_owned(),
            kind
        }]
    );
}

#[test]
fn a_directive_with_the_double_dash_is_named_without_it() {
    let issues = check_openvpn_config("client\n--up /x.sh\n");
    assert_eq!(issues[0].directive, "up");
}

#[test]
fn an_inline_block_of_a_public_file_is_not_an_issue() {
    let text = "client\n<ca>\nx\n</ca>\n<cert>\nx\n</cert>\n<extra-certs>\nx\n</extra-certs>\n<crl-verify>\nx\n</crl-verify>\n";
    assert_eq!(check_openvpn_config(text), vec![]);
}

#[test]
fn missing_tls_client_is_a_warning_at_line_0() {
    let issues = check_openvpn_config("remote vpn.example 1194\n");
    assert_eq!(
        issues,
        vec![ConfigIssue {
            line: 0,
            directive: "tls-client".to_owned(),
            kind: ConfigIssueKind::NoTlsClient
        }]
    );
    // `tls-client` alone is as good as `client`
    assert_eq!(check_openvpn_config("tls-client\n"), vec![]);
}

#[test]
fn issues_come_in_the_order_of_the_text_and_the_warning_comes_last() {
    let issues = check_openvpn_config("up /a\nfrob\ndev tun\n");
    let found: Vec<_> = issues.iter().map(|i| (i.line, i.kind)).collect();
    assert_eq!(
        found,
        [
            (1, ConfigIssueKind::Dangerous),
            (2, ConfigIssueKind::Unsupported),
            (3, ConfigIssueKind::DaemonOwned),
            (0, ConfigIssueKind::NoTlsClient),
        ]
    );
}

#[test]
fn an_issue_never_carries_the_content_of_a_block_or_an_argument() {
    let text = "client\nup /home/secret/path.sh\n<key>\nTOP-SECRET-KEY\n</key>\nremote \"TOP-SECRET-HOST 1\n";
    let shown = format!("{:?}", check_openvpn_config(text));
    assert!(!shown.contains("TOP-SECRET") && !shown.contains("secret/path"));
}

fn wireguard_profile() -> (Profile, Vec<(String, Secret)>) {
    let imported = import_wireguard(WG, "Homelab").unwrap();
    (imported.profile, imported.secrets)
}

#[test]
fn render_wireguard_without_keys_has_no_key_line() {
    let (profile, _) = wireguard_profile();
    let text = render_wireguard(profile.wireguard.as_ref().unwrap(), None);
    assert!(!text.contains("PrivateKey") && !text.contains("PresharedKey"));
    assert!(!text.contains("PRIVATE-KEY") && !text.contains("PSK-ONE"));
    assert!(text.contains("PublicKey = U0VFRC1QVUJMSUMtaG9tZWxhYi4uLi4uLi4uLi4uLi4="));
    assert!(text.contains("Address = 10.8.0.2/32, fd00::2/128"));
    assert!(text.contains("DNS = 10.8.0.1, example.org"));
    assert!(text.contains("Endpoint = casa.example:51820"));
    assert!(text.contains("PersistentKeepalive = 25"));
}

#[test]
fn render_wireguard_leaves_out_what_is_not_set() {
    let (profile, _) = wireguard_profile();
    let text = render_wireguard(profile.wireguard.as_ref().unwrap(), None);
    // the second peer has no endpoint and no keepalive
    let second = text.rsplit("[Peer]").next().unwrap();
    assert!(!second.contains("Endpoint") && !second.contains("PersistentKeepalive"));
    let mut bare = profile.wireguard.clone().unwrap();
    bare.dns.clear();
    bare.search_domains.clear();
    bare.mtu = None;
    let text = render_wireguard(&bare, None);
    assert!(!text.contains("DNS") && !text.contains("MTU"));
}

fn keys_of(secrets: &[(String, Secret)], peers: usize) -> WireGuardKeys {
    let get = |name: &str| {
        secrets
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    WireGuardKeys {
        private_key: get("wireguard.private_key").unwrap(),
        preshared: (0..peers)
            .map(|i| get(&format!("wireguard.peer.{i}.preshared_key")))
            .collect(),
    }
}

#[test]
fn render_wireguard_round_trips_through_the_import() {
    let (profile, secrets) = wireguard_profile();
    let wg = profile.wireguard.clone().unwrap();
    let keys = keys_of(&secrets, wg.peers.len());
    let text = render_wireguard(&wg, Some(&keys));
    assert!(text.contains("PrivateKey = PRIVATE-KEY") && text.contains("PresharedKey = PSK-ONE"));
    let again = import_wireguard(&text, "Homelab").unwrap();
    let back = again.profile.wireguard.unwrap();
    assert_eq!(serde_json_like(&back), serde_json_like(&wg));
    let mut before = secrets.clone();
    let mut after = again.secrets.clone();
    before.sort_by(|a, b| a.0.cmp(&b.0));
    after.sort_by(|a, b| a.0.cmp(&b.0));
    let names = |l: &[(String, Secret)]| {
        l.iter()
            .map(|(k, v)| (k.clone(), v.expose().to_owned()))
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&before), names(&after));
    assert!(again.warnings.is_empty());
}

/// The fields of a WireGuard profile as one comparable text.
fn serde_json_like(p: &WireGuardProfile) -> String {
    format!("{p:?}")
}
