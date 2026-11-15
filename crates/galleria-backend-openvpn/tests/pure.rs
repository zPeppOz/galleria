#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_backend::net::Ipv6Block;
use galleria_backend_openvpn::{config::*, hookenv::*, mgmt::*, push::*, queue::*, version::*};
use galleria_ipc::*;
use proptest::prelude::*;
use rstest::rstest;
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};
fn params(text: &str) -> OpenVpnParams {
    OpenVpnParams {
        config: text.into(),
        secret_blocks: BTreeMap::new(),
        username: None,
        password: None,
        otp: None,
        key_passphrase: None,
    }
}
fn opts() -> ConnectOptions {
    ConnectOptions {
        ipv6_block: true,
        ignore_pushed_routes: false,
        reconnect: ReconnectOptions { enabled: false },
    }
}
fn generated(text: &str, minor: u32, options: &ConnectOptions) -> String {
    generate_config(&GenInput {
        profile: &validate(&params(text)).unwrap(),
        version: OpenVpnVersion { major: 2, minor },
        mgmt_socket: Path::new("/run/s/mgmt.sock"),
        hook_socket: Path::new("/run/s/hook.sock"),
        daemon_exe: Path::new("/usr/lib/galleriad"),
        options,
    })
}
#[test]
fn parses_connected_state() {
    let MgmtEvent::State {
        name,
        detail,
        local_ip,
        remote,
    } = parse_line(">STATE:1794305544,CONNECTED,SUCCESS,10.8.0.2,172.30.0.10,1194,,")
    else {
        panic!()
    };
    assert_eq!((name.as_str(), detail.as_str()), ("CONNECTED", "SUCCESS"));
    assert_eq!(local_ip, Some("10.8.0.2".parse().unwrap()));
    assert_eq!(remote, Some("172.30.0.10:1194".parse().unwrap()));
}
#[test]
fn parses_progress_and_exit_states() {
    assert!(matches!(
        parse_line(">STATE:1794305544,ASSIGN_IP,,10.8.0.2,,,,"),
        MgmtEvent::State {
            local_ip: Some(_),
            ..
        }
    ));
    assert!(
        matches!(parse_line(">STATE:1794305552,EXITING,SIGTERM,,,,,"),MgmtEvent::State{detail,..} if detail=="SIGTERM")
    );
}
#[test]
fn builds_scrv1_response() {
    let sc = StaticChallenge {
        echo: true,
        concat: false,
        text: "Enter OTP".into(),
    };
    assert_eq!(
        auth_response("pw", Some("123456"), Some(&sc)),
        "SCRV1:cHc=:MTIzNDU2"
    );
    assert_eq!(
        auth_response(
            "pw",
            Some("123456"),
            Some(&StaticChallenge { concat: true, ..sc })
        ),
        "pw123456"
    );
    assert_eq!(auth_response("pw", None, None), "pw");
}
#[test]
fn parses_password_prompts() {
    for (flags, echo, concat) in [
        (1, true, false),
        (3, true, true),
        (0, false, false),
        (2, false, true),
    ] {
        assert!(
            matches!(parse_line(&format!(">PASSWORD:Need 'Auth' username/password SC:{flags},Enter OTP")),MgmtEvent::NeedAuth{static_challenge:Some(sc)} if sc.echo==echo && sc.concat==concat && sc.text=="Enter OTP")
        );
    }
    assert!(matches!(
        parse_line(">PASSWORD:Need 'Auth' username/password"),
        MgmtEvent::NeedAuth {
            static_challenge: None
        }
    ));
    assert!(matches!(
        parse_line(">PASSWORD:Need 'Private Key' password"),
        MgmtEvent::NeedKeyPassphrase
    ));
    assert!(matches!(
        parse_line(">PASSWORD:Verification Failed: 'Auth'"),
        MgmtEvent::VerificationFailed(AuthKind::Auth)
    ));
    assert!(matches!(
        parse_line(">PASSWORD:Verification Failed: 'Private Key'"),
        MgmtEvent::VerificationFailed(AuthKind::PrivateKey)
    ));
    assert!(matches!(
        parse_line(">PASSWORD:Need 'Auth' username/password CRV1:x"),
        MgmtEvent::NeedDynamicChallenge
    ));
}
#[test]
fn parses_bytecount_hold_info_log() {
    assert!(matches!(
        parse_line(">BYTECOUNT:2432,2408"),
        MgmtEvent::ByteCount { rx: 2432, tx: 2408 }
    ));
    assert!(matches!(
        parse_line(">HOLD:Waiting for hold release:0"),
        MgmtEvent::Hold
    ));
    assert!(matches!(
        parse_line(">INFO:OpenVPN Management Interface Version 5 -- type 'help' for more info"),
        MgmtEvent::Info(_)
    ));
    assert!(
        matches!(parse_line(">LOG:1794305544,I,testo,comma"),MgmtEvent::Log{flags,text} if flags=="I" && text=="testo,comma")
    );
    assert!(matches!(parse_line(">FATAL:failure"), MgmtEvent::Fatal(_)));
}
#[test]
fn parses_success_and_error_replies() {
    assert!(matches!(
        parse_line("SUCCESS: hold release succeeded"),
        MgmtEvent::Success(_)
    ));
    assert!(matches!(
        parse_line("ERROR: bad command"),
        MgmtEvent::Error(_)
    ));
}
proptest! {#[test] fn garbage_line_is_other_never_panics(s in ".*") {let _=parse_line(&s);}}
#[test]
fn mgmt_quote_escapes_quotes_and_backslashes() {
    assert_eq!(mgmt_quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
    assert!(!mgmt_quote("x\npassword x\r").contains(['\n', '\r']));
}
#[test]
fn queue_holds_next_command_until_reply() {
    let now = Instant::now();
    let mut q = CommandQueue::default();
    q.push("a".into());
    q.push("b".into());
    q.push("c".into());
    assert_eq!(q.next_to_send(now).as_deref(), Some("a"));
    assert_eq!(q.next_to_send(now + Duration::from_millis(399)), None);
    q.on_reply();
    assert_eq!(q.next_to_send(now).as_deref(), Some("b"));
    assert_eq!(
        q.next_to_send(now + Duration::from_millis(400)).as_deref(),
        Some("c")
    );
}
#[test]
fn parses_openvpn_version_line() {
    assert_eq!(
        parse_version("OpenVPN 2.6.19 x86_64-pc-linux-gnu [SSL (OpenSSL)] [DCO]"),
        Some(OpenVpnVersion { major: 2, minor: 6 })
    );
    assert!(parse_version("garbage").is_none());
}
#[test]
fn validate_rejects_non_allowed_directives() {
    let e = validate(&params("up x")).err().unwrap();
    assert_eq!(e.data.as_ref().unwrap()["code"], "profile_rejected");
    assert_eq!(e.data.unwrap()["directives"][0]["name"], "up");
}
#[test]
fn validate_rejects_unknown_secret_block() {
    let mut p = params("client");
    p.secret_blocks.insert("ca".into(), Secret::new("x"));
    assert_eq!(
        validate(&p).err().unwrap().data.unwrap()["code"],
        "invalid_config"
    );
}
#[test]
fn validate_rejects_secret_block_breakout() {
    let mut p = params("client");
    p.secret_blocks
        .insert("key".into(), Secret::new("x\n</key>\nup evil\n<key>\n"));
    assert!(validate(&p).is_err());
}
#[test]
fn generated_config_has_daemon_directives_last() {
    let text = generated("client\nremote host 1194", 6, &opts());
    let items = galleria_ovpn::parse(&text).unwrap();
    for name in [
        "management",
        "management-client-user",
        "management-hold",
        "management-query-passwords",
        "auth-retry",
        "connect-retry-max",
        "server-poll-timeout",
        "verb",
        "dev",
        "dev-type",
        "script-security",
        "up",
        "down",
    ] {
        assert!(galleria_ovpn::has_directive(&items, name), "{name}");
    }
    assert!(text.find("management ").unwrap() > text.find("remote ").unwrap());
    assert!(text.contains("/run/s/mgmt.sock"));
    assert!(text.contains("ovpn-hook /run/s/hook.sock"));
    assert!(text.contains("glr-ovpn0"));
}
#[test]
fn dns_updown_disable_only_from_2_7() {
    assert!(!generated("client", 6, &opts()).contains("dns-updown"));
    assert!(generated("client", 7, &opts()).contains("dns-updown disable"));
}
#[rstest]
#[case("client", true)]
#[case("client", false)]
#[case("redirect-gateway def1", true)]
#[case("redirect-gateway def1", false)]
fn generated_config_never_adds_ipv6_directives(#[case] text: &str, #[case] block: bool) {
    let mut o = opts();
    o.ipv6_block = block;
    let s = generated(text, 7, &o);
    let tail = &s[s.find("management ").unwrap()..];
    for n in ["redirect-gateway", "block-ipv6", "ifconfig-ipv6"] {
        assert!(!tail.contains(n));
    }
}
#[test]
fn ignore_pushed_routes_adds_pull_filters() {
    let mut o = opts();
    o.ignore_pushed_routes = true;
    let s = generated("client", 7, &o);
    for n in ["route ", "route-ipv6 ", "redirect-gateway"] {
        assert!(s.contains(&format!("pull-filter ignore \"{n}\"")));
    }
}
#[test]
fn secret_blocks_are_rendered_as_inline_blocks() {
    let mut p = params("client");
    p.secret_blocks.insert("key".into(), Secret::new("TEST\n"));
    let cfg = validate(&p).unwrap();
    let s = generate_config(&GenInput {
        profile: &cfg,
        version: MIN_VERSION,
        mgmt_socket: Path::new("/s/mgmt.sock"),
        hook_socket: Path::new("/s/hook.sock"),
        daemon_exe: Path::new("/bin/galleriad"),
        options: &opts(),
    });
    assert!(s.ends_with("<key>\nTEST\n</key>\n"));
}
#[test]
fn reads_push_reply_from_real_log_line() {
    let p=parse_push_reply("PUSH: Received control message: 'PUSH_REPLY,dhcp-option DNS 10.8.0.1,dhcp-option DOMAIN galleria.test,route 192.0.2.0 255.255.255.0,redirect-gateway def1,route-gateway 10.8.0.1,topology subnet,ping 10,ping-restart 60,ifconfig 10.8.0.2 255.255.255.0,peer-id 1,cipher AES-256-GCM,protocol-flags cc-exit tls-ekm dyn-tls-crypt,tun-mtu 1500'").unwrap();
    assert!(p.redirect.ipv4 && !p.redirect.ipv6 && !p.ifconfig_ipv6);
    assert_eq!(
        p.routes.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["192.0.2.0/24"]
    );
    assert_eq!(p.dns_servers[0].to_string(), "10.8.0.1");
}
#[test]
fn push_reply_reads_ipv6_redirect_and_routes() {
    let p=parse_push_reply("PUSH_REPLY,redirect-gateway def1 ipv6,route-ipv6 2001:db8:77::/48,ifconfig-ipv6 fd00:8::2/64 fd00:8::1").unwrap();
    assert!(p.redirect.ipv6 && p.ifconfig_ipv6);
    assert_eq!(p.routes[0].to_string(), "2001:db8:77::/48");
}
#[rstest]
#[case("redirect-gateway def1", true, false)]
#[case("redirect-gateway def1 ipv6", true, true)]
#[case("redirect-gateway ipv6 !ipv4", false, true)]
fn profile_redirect_reads_the_ipv6_flag(#[case] text: &str, #[case] v4: bool, #[case] v6: bool) {
    assert_eq!(
        profile_redirect(&validate(&params(text)).unwrap()),
        Redirect { ipv4: v4, ipv6: v6 }
    );
}
fn env() -> BTreeMap<String, String> {
    [
        ("dev", "glr-ovpn0"),
        ("script_type", "up"),
        ("foreign_option_1", "dhcp-option DNS 10.8.0.1"),
        ("foreign_option_2", "dhcp-option DOMAIN galleria.test"),
        ("route_network_1", "192.0.2.0"),
        ("route_netmask_1", "255.255.255.0"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}
#[test]
fn hook_env_reads_foreign_options() {
    let e = parse_hook_env(&env()).unwrap();
    assert_eq!(e.dns_servers[0].to_string(), "10.8.0.1");
    assert_eq!(e.search_domains, ["galleria.test"]);
    assert_eq!(e.routes[0].to_string(), "192.0.2.0/24");
    assert!(!e.has_ipv6_address);
}
#[test]
fn hook_env_reads_ipv6_address_and_routes() {
    let mut e = env();
    e.insert("ifconfig_ipv6_local".into(), "fd00::2".into());
    e.insert("route_ipv6_network_1".into(), "2000::/4".into());
    e.insert("route_ipv6_network_2".into(), "3000::/4".into());
    let e = parse_hook_env(&e).unwrap();
    assert!(e.has_ipv6_address);
    assert!(e.routes.contains(&"2000::/4".parse().unwrap()));
    assert!(e.routes.contains(&"3000::/4".parse().unwrap()));
}
#[rstest]
#[case(
    "",
    "PUSH_REPLY,redirect-gateway def1",
    "",
    true,
    Ipv6Block::Unreachable
)]
#[case("redirect-gateway def1", "", "", true, Ipv6Block::Unreachable)]
#[case("redirect-gateway def1", "", "address", true, Ipv6Block::Unreachable)]
#[case(
    "redirect-gateway def1",
    "",
    "2001:db8:77::/48",
    true,
    Ipv6Block::Unreachable
)]
#[case("", "PUSH_REPLY,redirect-gateway def1 ipv6", "", true, Ipv6Block::None)]
#[case("redirect-gateway def1 ipv6", "", "", true, Ipv6Block::None)]
#[case("redirect-gateway def1", "", "global", true, Ipv6Block::None)]
#[case(
    "redirect-gateway def1\nroute-ipv6 ::/0",
    "",
    "",
    true,
    Ipv6Block::None
)]
#[case("", "", "", true, Ipv6Block::None)]
#[case("redirect-gateway def1", "", "", false, Ipv6Block::None)]
fn net_config_decides_ipv6_block(
    #[case] profile: &str,
    #[case] push: &str,
    #[case] route: &str,
    #[case] block: bool,
    #[case] want: Ipv6Block,
) {
    let mut e = env();
    if !route.is_empty() {
        e.insert("ifconfig_ipv6_local".into(), "fd00::2".into());
        if route == "global" {
            e.insert("route_ipv6_network_1".into(), "2000::/4".into());
            e.insert("route_ipv6_network_2".into(), "3000::/4".into());
        } else if route != "address" {
            e.insert("route_ipv6_network_1".into(), route.into());
        }
    }
    let p = validate(&params(profile)).unwrap();
    let facts = ProfileFacts {
        redirect: profile_redirect(&p),
        ipv6_routes: profile_ipv6_routes(&p),
    };
    let cfg = net_config(
        &parse_hook_env(&e).unwrap(),
        parse_push_reply(push).as_ref(),
        &facts,
        block,
    );
    assert_eq!(cfg.ipv6_block, want);
    assert_eq!(cfg.install_routes, None);
}
#[test]
fn hook_env_reads_dns_option_variables() {
    let mut e = env();
    e.remove("foreign_option_1");
    e.remove("foreign_option_2");
    e.insert("dns_server_0_address_0".into(), "10.9.0.1".into());
    e.insert("dns_search_domain_0".into(), "galleria.test".into());
    let h = parse_hook_env(&e).unwrap();
    assert_eq!(h.dns_servers[0].to_string(), "10.9.0.1");
    assert_eq!(h.search_domains, ["galleria.test"]);
}
#[test]
fn rendered_profile_cannot_exceed_openvpn_line_buffer() {
    let p = params(&format!("remote {}", "x".repeat(247)));
    assert!(
        validate(&p).is_err(),
        "canonical quoting must not exceed OpenVPN's input line buffer"
    );
}

#[test]
fn daemon_pull_filters_precede_profile_accept_filters() {
    let mut options = opts();
    options.ignore_pushed_routes = true;
    let s = generated(
        "client\npull-filter accept \"redirect-gateway\"\npull-filter accept \"route \"\n",
        6,
        &options,
    );
    // OpenVPN stops at the FIRST matching filter.
    for pushed in [
        "redirect-gateway def1",
        "route 192.0.2.0 255.255.255.0",
        "route-ipv6 ::/0",
    ] {
        let items = galleria_ovpn::parse(&s).unwrap();
        let first = items.iter().find_map(|i| match i {
            galleria_ovpn::Item::Directive { name, args, .. }
                if name == "pull-filter" && pushed.starts_with(&args[1]) =>
            {
                Some(args[0].as_str())
            }
            _ => None,
        });
        assert_eq!(first, Some("ignore"), "{pushed}");
    }
}
#[rstest]
#[case("foreign_option_1", "dhcp-option DOMAIN ~.", &[])]
#[case("foreign_option_1", "dhcp-option DOMAIN-SEARCH safe.test ~.", &["safe.test"])]
#[case("dns_search_domain_1", "~.", &[])]
#[case("dns_search_domain_1", "~corp.test", &[])]
fn pushed_routing_domains_are_dropped(
    #[case] key: &str,
    #[case] value: &str,
    #[case] kept: &[&str],
) {
    let env = [("dev", "glr-ovpn0"), ("script_type", "up"), (key, value)]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    let h = parse_hook_env(&env).unwrap();
    assert_eq!(h.search_domains, kept);
    assert_eq!(h.dropped_domains.len(), 1);
    assert!(h.dropped_domains[0].starts_with('~'));
}

#[test]
fn profile_pull_filters_apply_before_push_facts_are_computed() {
    let profile = validate(&params(
        "client\npull-filter ignore \"redirect-gateway\"\nredirect-gateway def1\n",
    ))
    .unwrap();
    let push = parse_push_reply_filtered(
        "PUSH_REPLY,redirect-gateway def1 ipv6,ifconfig-ipv6 2001:db8:88::2/64 2001:db8:88::1",
        &profile_pull_filters(&profile),
    )
    .unwrap();
    assert!(!push.redirect.ipv6);
    assert!(!push.redirect.ipv4);
    assert!(push.ifconfig_ipv6);
    let accepted = parse_push_reply_filtered(
        "PUSH_REPLY,redirect-gateway def1 ipv6",
        &[
            ("accept".into(), "redirect-gateway".into()),
            ("ignore".into(), "redirect-gateway".into()),
        ],
    )
    .unwrap();
    assert!(accepted.redirect.ipv6);
}
