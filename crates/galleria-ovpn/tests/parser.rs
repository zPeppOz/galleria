#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ovpn::*;
use proptest::prelude::*;
use rstest::rstest;
#[test]
fn parses_quoted_arguments() {
    let p = parse("static-challenge \"Enter OTP\" 1\n").unwrap();
    assert!(matches!(&p[0], Item::Directive {args,..} if args==&["Enter OTP", "1"]));
    assert!(
        matches!(&parse("remote 'a b' 1194\n").unwrap()[0], Item::Directive {args,..} if args==&["a b", "1194"])
    );
}
#[test]
fn skips_comments_and_blank_lines() {
    assert!(parse("\n# x\n ; x\n\t\n").unwrap().is_empty());
}
#[test]
fn strips_leading_double_dash() {
    assert!(has_directive(&parse("--nobind").unwrap(), "nobind"));
}
#[test]
fn parses_crlf_and_bom() {
    assert_eq!(
        parse(include_str!("fixtures/base.ovpn")).unwrap(),
        parse(include_str!("fixtures/crlf.ovpn")).unwrap()
    );
}
#[test]
fn block_body_is_preserved_byte_for_byte() {
    assert!(
        matches!(&parse("<ca>\n  PEM\r\n\n</ca>\n").unwrap()[0], Item::Block {body,..} if body=="  PEM\r\n\n")
    );
}
#[test]
fn unterminated_block_is_an_error() {
    assert!(matches!(
        parse("<key>\nx"),
        Err(ParseError::UnterminatedBlock { line: 1 })
    ));
}
#[test]
fn connection_blocks_are_classified_inside() {
    let p = parse("<connection>\nup x\n</connection>\n").unwrap();
    let e = check_strict(&p).unwrap_err();
    assert_eq!(e[0].class, Class::Dangerous);
}
#[test]
fn uppercase_name_is_unknown() {
    assert_eq!(classify("UP", &[]), Class::Unknown);
}
#[test]
fn malicious_fixture_is_rejected_by_strict() {
    let p = parse(include_str!("fixtures/malicious.ovpn")).unwrap();
    let got: std::collections::BTreeSet<_> = check_strict(&p)
        .unwrap_err()
        .into_iter()
        .map(|r| r.name)
        .collect();
    let want="script-security up down route-up ipchange plugin management status writepid setenv verb dev".split_whitespace().map(String::from).collect();
    assert_eq!(got, want);
}
#[test]
fn import_filter_warns_on_dangerous_not_on_daemon_owned() {
    let f = import_filter(parse("up x\nverb 3\nclient\n").unwrap());
    assert_eq!(f.warnings.len(), 1);
    assert_eq!(f.warnings[0].name, "up");
    assert!(check_strict(&f.items).is_ok());
}
#[test]
fn render_parse_roundtrip() {
    let p=parse("# a\nclient\n\nstatic-challenge \"a \\\" b\\\\c\" 1\n<ca>\nPEM\n</ca>\n<connection>\nremote foo 1194\n</connection>\n").unwrap();
    assert_eq!(parse(&render(&p)).unwrap(), p);
}
proptest! {
 #[test]
 fn import_filter_then_strict_always_passes(names in prop::collection::vec("[a-z-]{1,24}",0..40)) {
    let p=parse(&names.join("\n")).unwrap(); prop_assert!(check_strict(&import_filter(p).items).is_ok());
 }
}
#[rstest]
#[case("auth-user-pass", &[], Class::Allowed)]
#[case("auth-user-pass", &["file"], Class::Dangerous)]
#[case("askpass", &["file"], Class::Dangerous)]
#[case("http-proxy", &["host","80"], Class::Allowed)]
#[case("http-proxy", &["host","80","basic"], Class::Allowed)]
#[case("http-proxy", &["host","80","file"], Class::Dangerous)]
#[case("http-proxy", &["host","80","none","basic"], Class::Dangerous)]
#[case("socks-proxy", &["host"], Class::Allowed)]
#[case("socks-proxy", &["host","80","file"], Class::Dangerous)]
#[case("ca", &["[inline]"], Class::Allowed)]
#[case("ca", &["file"], Class::FileRef)]
fn classify_argument_variants(#[case] name: &str, #[case] args: &[&str], #[case] want: Class) {
    assert_eq!(
        classify(
            name,
            &args.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        ),
        want
    );
}

#[rstest]
#[case("client", Class::Allowed)]
#[case("tls-client", Class::Allowed)]
#[case("pull", Class::Allowed)]
#[case("dev-type", Class::Allowed)]
#[case("proto", Class::Allowed)]
#[case("remote", Class::Allowed)]
#[case("remote-random", Class::Allowed)]
#[case("remote-random-hostname", Class::Allowed)]
#[case("resolv-retry", Class::Allowed)]
#[case("nobind", Class::Allowed)]
#[case("bind", Class::Allowed)]
#[case("lport", Class::Allowed)]
#[case("rport", Class::Allowed)]
#[case("port", Class::Allowed)]
#[case("float", Class::Allowed)]
#[case("cipher", Class::Allowed)]
#[case("data-ciphers", Class::Allowed)]
#[case("data-ciphers-fallback", Class::Allowed)]
#[case("ncp-ciphers", Class::Allowed)]
#[case("auth", Class::Allowed)]
#[case("auth-user-pass", Class::Allowed)]
#[case("auth-nocache", Class::Allowed)]
#[case("static-challenge", Class::Allowed)]
#[case("remote-cert-tls", Class::Allowed)]
#[case("verify-x509-name", Class::Allowed)]
#[case("tls-version-min", Class::Allowed)]
#[case("tls-version-max", Class::Allowed)]
#[case("tls-cipher", Class::Allowed)]
#[case("tls-ciphersuites", Class::Allowed)]
#[case("tls-groups", Class::Allowed)]
#[case("tls-cert-profile", Class::Allowed)]
#[case("key-direction", Class::Allowed)]
#[case("comp-lzo", Class::Allowed)]
#[case("compress", Class::Allowed)]
#[case("allow-compression", Class::Allowed)]
#[case("tun-mtu", Class::Allowed)]
#[case("mssfix", Class::Allowed)]
#[case("fragment", Class::Allowed)]
#[case("mtu-disc", Class::Allowed)]
#[case("sndbuf", Class::Allowed)]
#[case("rcvbuf", Class::Allowed)]
#[case("keepalive", Class::Allowed)]
#[case("ping", Class::Allowed)]
#[case("ping-restart", Class::Allowed)]
#[case("ping-exit", Class::Allowed)]
#[case("ping-timer-rem", Class::Allowed)]
#[case("inactive", Class::Allowed)]
#[case("reneg-sec", Class::Allowed)]
#[case("reneg-bytes", Class::Allowed)]
#[case("reneg-pkts", Class::Allowed)]
#[case("hand-window", Class::Allowed)]
#[case("tran-window", Class::Allowed)]
#[case("explicit-exit-notify", Class::Allowed)]
#[case("redirect-gateway", Class::Allowed)]
#[case("redirect-private", Class::Allowed)]
#[case("route", Class::Allowed)]
#[case("route-ipv6", Class::Allowed)]
#[case("route-nopull", Class::Allowed)]
#[case("route-metric", Class::Allowed)]
#[case("route-gateway", Class::Allowed)]
#[case("pull-filter", Class::Allowed)]
#[case("dhcp-option", Class::Allowed)]
#[case("dns", Class::Allowed)]
#[case("topology", Class::Allowed)]
#[case("block-ipv6", Class::Allowed)]
#[case("mute-replay-warnings", Class::Allowed)]
#[case("replay-window", Class::Allowed)]
#[case("http-proxy-option", Class::Allowed)]
#[case("dev", Class::DaemonOwned)]
#[case("verb", Class::DaemonOwned)]
#[case("mute", Class::DaemonOwned)]
#[case("auth-retry", Class::DaemonOwned)]
#[case("connect-retry", Class::DaemonOwned)]
#[case("connect-retry-max", Class::DaemonOwned)]
#[case("connect-timeout", Class::DaemonOwned)]
#[case("server-poll-timeout", Class::DaemonOwned)]
#[case("persist-tun", Class::DaemonOwned)]
#[case("persist-key", Class::DaemonOwned)]
#[case("script-security", Class::DaemonOwned)]
#[case("block-outside-dns", Class::DaemonOwned)]
#[case("ignore-unknown-option", Class::DaemonOwned)]
#[case("management", Class::DaemonOwned)]
#[case("management-hold", Class::DaemonOwned)]
#[case("management-client-user", Class::DaemonOwned)]
#[case("up", Class::Dangerous)]
#[case("down", Class::Dangerous)]
#[case("route-up", Class::Dangerous)]
#[case("route-pre-down", Class::Dangerous)]
#[case("ipchange", Class::Dangerous)]
#[case("tls-verify", Class::Dangerous)]
#[case("client-connect", Class::Dangerous)]
#[case("client-disconnect", Class::Dangerous)]
#[case("learn-address", Class::Dangerous)]
#[case("auth-user-pass-verify", Class::Dangerous)]
#[case("plugin", Class::Dangerous)]
#[case("setenv", Class::Dangerous)]
#[case("setenv-safe", Class::Dangerous)]
#[case("config", Class::Dangerous)]
#[case("cd", Class::Dangerous)]
#[case("chroot", Class::Dangerous)]
#[case("user", Class::Dangerous)]
#[case("group", Class::Dangerous)]
#[case("log", Class::Dangerous)]
#[case("log-append", Class::Dangerous)]
#[case("syslog", Class::Dangerous)]
#[case("status", Class::Dangerous)]
#[case("writepid", Class::Dangerous)]
#[case("daemon", Class::Dangerous)]
#[case("engine", Class::Dangerous)]
#[case("providers", Class::Dangerous)]
#[case("dns-updown", Class::Dangerous)]
#[case("iproute", Class::Dangerous)]
#[case("tmp-dir", Class::Dangerous)]
#[case("tls-export-cert", Class::Dangerous)]
#[case("up-restart", Class::Dangerous)]
#[case("up-delay", Class::Dangerous)]
#[case("echo", Class::Dangerous)]
#[case("http-proxy-user-pass", Class::Dangerous)]
#[case("ca", Class::FileRef)]
#[case("cert", Class::FileRef)]
#[case("key", Class::FileRef)]
#[case("tls-auth", Class::FileRef)]
#[case("tls-crypt", Class::FileRef)]
#[case("tls-crypt-v2", Class::FileRef)]
#[case("pkcs12", Class::FileRef)]
#[case("extra-certs", Class::FileRef)]
#[case("crl-verify", Class::FileRef)]
fn classify_matches_the_list(#[case] name: &str, #[case] want: Class) {
    assert_eq!(classify(name, &[]), want);
}
#[test]
fn non_file_inline_blocks_cannot_bypass_allowlist() {
    assert!(check_strict(&parse("<client>\nup evil\n</client>\n").unwrap()).is_err());
}
#[rstest]
#[case("allow-pull-fqdn")]
#[case("allow-recursive-routing")]
#[case("client-nat")]
#[case("proto-force")]
#[case("local")]
#[case("link-mtu")]
#[case("mtu-test")]
#[case("passtos")]
#[case("socket-flags")]
#[case("tcp-nodelay")]
#[case("max-packet-size")]
#[case("session-timeout")]
fn manual_client_and_link_options_are_supported(#[case] name: &str) {
    assert_eq!(classify(name, &[]), Class::Allowed);
}
#[test]
#[ignore = "requires lab/gen-pki.sh generated profiles"]
fn lab_profiles_use_known_directives() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lab/out");
    let mut count = 0;
    for path in std::fs::read_dir(root).unwrap().map(|p| p.unwrap().path()) {
        if path.extension().is_some_and(|e| e == "ovpn") && path.file_stem().unwrap() != "malicious"
        {
            let p = parse(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert!(check_strict(&p).is_ok());
            count += 1;
        }
    }
    assert_eq!(count, 7);
}
