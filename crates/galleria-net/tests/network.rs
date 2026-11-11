#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_backend::net::{Ipv6Block, NetConfig};
use galleria_net::*;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
fn config() -> NetConfig {
    NetConfig {
        interface: "glr-wg0".into(),
        dns_servers: vec!["10.99.0.1".parse().unwrap()],
        search_domains: vec!["corp.test".into()],
        routes: vec!["0.0.0.0/0".parse().unwrap()],
        install_routes: Some(51820),
        ipv6_block: Ipv6Block::Unreachable,
    }
}
fn applied(cfg: &NetConfig) -> Vec<String> {
    plan(cfg).iter().map(|s| s.apply.to_string()).collect()
}
fn undos(cfg: &NetConfig) -> Vec<String> {
    plan(cfg)
        .into_iter()
        .filter_map(|s| s.undo)
        .map(|s| s.to_string())
        .collect()
}
#[test]
fn plan_for_wireguard_full_tunnel_with_dns() {
    assert_eq!(
        applied(&config()),
        [
            "ip -4 route add 0.0.0.0/0 dev glr-wg0 table 51820",
            "ip -4 rule add not fwmark 51820 table 51820",
            "ip -4 rule add table main suppress_prefixlen 0",
            "sysctl -q -w net.ipv4.conf.all.src_valid_mark=1",
            "ip -6 route replace unreachable ::/1",
            "ip -6 route replace unreachable 8000::/1",
            "resolvectl dns glr-wg0 10.99.0.1",
            "resolvectl domain glr-wg0 corp.test ~.",
            "resolvectl default-route glr-wg0 yes",
        ]
    );
}
#[test]
fn undo_commands_mirror_the_plan() {
    assert_eq!(
        undos(&config()),
        [
            "ip -4 rule del not fwmark 51820 table 51820",
            "ip -4 rule del table main suppress_prefixlen 0",
            "ip -6 route del unreachable ::/1",
            "ip -6 route del unreachable 8000::/1",
            "resolvectl revert glr-wg0",
        ]
    );
}
#[test]
fn split_tunnel_adds_specific_routes_only() {
    let mut cfg = config();
    cfg.routes = vec!["192.168.1.0/24".parse().unwrap()];
    cfg.ipv6_block = Ipv6Block::None;
    assert_eq!(
        applied(&cfg),
        [
            "ip -4 route add 192.168.1.0/24 dev glr-wg0",
            "resolvectl dns glr-wg0 10.99.0.1",
            "resolvectl domain glr-wg0 corp.test",
            "resolvectl default-route glr-wg0 yes"
        ]
    );
}
#[test]
fn ipv6_routes_use_dash_six() {
    let mut cfg = config();
    cfg.routes = vec!["fd00::/8".parse().unwrap()];
    cfg.ipv6_block = Ipv6Block::None;
    cfg.dns_servers.clear();
    assert_eq!(applied(&cfg), ["ip -6 route add fd00::/8 dev glr-wg0"]);
}
#[test]
fn backend_installed_routes_touch_only_dns() {
    let mut cfg = config();
    cfg.install_routes = None;
    cfg.ipv6_block = Ipv6Block::None;
    assert_eq!(
        applied(&cfg),
        [
            "resolvectl dns glr-wg0 10.99.0.1",
            "resolvectl domain glr-wg0 corp.test ~.",
            "resolvectl default-route glr-wg0 yes"
        ]
    );
}
#[test]
fn no_dns_means_no_resolvectl() {
    let mut cfg = config();
    cfg.dns_servers.clear();
    assert!(
        plan(&cfg)
            .iter()
            .all(|s| s.apply.program != Program::Resolvectl)
    );
}
#[test]
fn no_block_adds_no_ipv6_routes() {
    let mut cfg = config();
    cfg.ipv6_block = Ipv6Block::None;
    assert!(applied(&cfg).iter().all(|s| !s.contains("unreachable")));
}
#[test]
fn openvpn_block_is_unreachable_routes_without_other_routes() {
    let mut cfg = config();
    cfg.interface = "glr-ovpn0".into();
    cfg.install_routes = None;
    cfg.dns_servers.clear();
    assert_eq!(
        applied(&cfg),
        [
            "ip -6 route replace unreachable ::/1",
            "ip -6 route replace unreachable 8000::/1"
        ]
    );
    assert_eq!(
        undos(&cfg),
        [
            "ip -6 route del unreachable ::/1",
            "ip -6 route del unreachable 8000::/1"
        ]
    );
}
#[derive(Default)]
struct Calls {
    commands: Vec<String>,
    fail_at: Option<usize>,
    missing: bool,
    fail_undo: bool,
    journal_seen: bool,
}
#[derive(Clone)]
struct Runner {
    calls: Arc<Mutex<Calls>>,
    journal: PathBuf,
}
impl CommandRunner for Runner {
    fn run(&self, cmd: &Cmd) -> Result<(), NetError> {
        let mut calls = self.calls.lock().unwrap();
        if calls.commands.is_empty() {
            calls.journal_seen = self.journal.exists();
        }
        calls.commands.push(cmd.to_string());
        if calls.fail_at == Some(calls.commands.len())
            || calls.missing
            || (calls.fail_undo && (cmd.args.iter().any(|s| s == "del" || s == "revert")))
        {
            Err(NetError::Command {
                cmd: cmd.to_string(),
                stderr: if calls.missing {
                    "RTNETLINK answers: No such file or directory"
                } else {
                    "synthetic failure"
                }
                .into(),
            })
        } else {
            Ok(())
        }
    }
}
struct Harness {
    _tmp: tempfile::TempDir,
    journal: PathBuf,
    calls: Arc<Mutex<Calls>>,
    net: NetManager<Runner>,
}
impl Harness {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("net-journal.json");
        let calls = Arc::new(Mutex::new(Calls::default()));
        let net = NetManager::new(
            Runner {
                calls: calls.clone(),
                journal: journal.clone(),
            },
            journal.clone(),
        );
        Self {
            _tmp: tmp,
            journal,
            calls,
            net,
        }
    }
    fn commands(&self) -> Vec<String> {
        self.calls.lock().unwrap().commands.clone()
    }
}
#[test]
fn apply_after_teardown_works_again() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    h.net.teardown().unwrap();
    let mut b = config();
    b.interface = "glr-ovpn0".into();
    b.install_routes = None;
    h.net.apply(&b).unwrap();
    let journal = std::fs::read_to_string(&h.journal).unwrap();
    assert!(journal.contains("glr-ovpn0"));
    assert!(!journal.contains("glr-wg0"));
    h.net.teardown().unwrap();
    assert!(!h.journal.exists());
}
#[test]
fn apply_writes_journal_before_first_command() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    assert!(h.calls.lock().unwrap().journal_seen);
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&h.journal).unwrap()).unwrap();
    assert_eq!(value["undo"].as_array().unwrap().len(), 5);
}
#[test]
fn apply_failure_rolls_back_what_ran() {
    let h = Harness::new();
    h.calls.lock().unwrap().fail_at = Some(3);
    assert!(h.net.apply(&config()).is_err());
    assert_eq!(
        h.commands(),
        [
            "ip -4 route add 0.0.0.0/0 dev glr-wg0 table 51820",
            "ip -4 rule add not fwmark 51820 table 51820",
            "ip -4 rule add table main suppress_prefixlen 0",
            "ip -4 rule del table main suppress_prefixlen 0",
            "ip -4 rule del not fwmark 51820 table 51820"
        ]
    );
    assert!(!h.journal.exists());
}
#[test]
fn resolvectl_failure_is_dns_unavailable() {
    let h = Harness::new();
    h.calls.lock().unwrap().fail_at = Some(7);
    assert!(matches!(
        h.net.apply(&config()),
        Err(NetError::DnsUnavailable(_))
    ));
    assert!(!h.journal.exists());
}
#[test]
fn teardown_runs_undos_in_reverse_and_removes_journal() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    h.calls.lock().unwrap().commands.clear();
    h.net.teardown().unwrap();
    assert_eq!(
        h.commands(),
        [
            "resolvectl revert glr-wg0",
            "ip -6 route del unreachable 8000::/1",
            "ip -6 route del unreachable ::/1",
            "ip -4 rule del table main suppress_prefixlen 0",
            "ip -4 rule del not fwmark 51820 table 51820"
        ]
    );
    assert!(!h.journal.exists());
}
#[test]
fn teardown_twice_is_a_noop() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    h.net.teardown().unwrap();
    let before = h.commands();
    h.net.teardown().unwrap();
    assert_eq!(h.commands(), before);
}
#[test]
fn teardown_tolerates_missing_objects() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    h.calls.lock().unwrap().commands.clear();
    h.calls.lock().unwrap().missing = true;
    h.net.teardown().unwrap();
    assert_eq!(h.commands().len(), 5);
    assert!(!h.journal.exists());
}
#[test]
fn recover_replays_journal_then_fixed_cleanup() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    h.calls.lock().unwrap().commands.clear();
    let recovered = NetManager::new(
        Runner {
            calls: h.calls.clone(),
            journal: h.journal.clone(),
        },
        h.journal.clone(),
    );
    recovered.recover().unwrap();
    let calls = h.commands();
    assert_eq!(calls[0], "resolvectl revert glr-wg0");
    assert_eq!(
        &calls[5..],
        &[
            "ip link del glr-wg0",
            "ip link del glr-ovpn0",
            "ip -4 rule del not fwmark 51820 table 51820",
            "ip -4 rule del table main suppress_prefixlen 0",
            "ip -6 rule del not fwmark 51820 table 51820",
            "ip -6 rule del table main suppress_prefixlen 0"
        ]
    );
    assert!(!h.journal.exists());
}
#[test]
fn recover_without_journal_still_runs_fixed_cleanup() {
    let h = Harness::new();
    h.net.recover().unwrap();
    assert_eq!(h.commands().len(), 6);
    assert_eq!(h.commands()[0], "ip link del glr-wg0");
    assert!(!h.journal.exists());
}
#[test]
fn failed_cleanup_keeps_journal_and_attempts_every_undo() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    h.calls.lock().unwrap().commands.clear();
    h.calls.lock().unwrap().fail_undo = true;
    assert!(h.net.teardown().is_err());
    assert_eq!(h.commands().len(), 5);
    assert!(h.journal.exists());
    h.calls.lock().unwrap().fail_undo = false;
    h.net.teardown().unwrap();
    assert!(!h.journal.exists());
}
#[test]
fn apply_does_not_overwrite_an_active_journal() {
    let h = Harness::new();
    h.net.apply(&config()).unwrap();
    let before = std::fs::read(&h.journal).unwrap();
    let count = h.commands().len();
    assert!(h.net.apply(&config()).is_err());
    assert_eq!(h.commands().len(), count);
    assert_eq!(std::fs::read(&h.journal).unwrap(), before);
}
#[test]
fn invalid_config_cannot_generate_host_commands() {
    for invalid in ["eth0", "glr-wg0;reboot", "--help"] {
        let h = Harness::new();
        let mut cfg = config();
        cfg.interface = invalid.into();
        assert!(h.net.apply(&cfg).is_err());
        assert!(h.commands().is_empty());
    }
    for domain in ["--help", "bad\ncommand", "bad domain", "../x"] {
        let h = Harness::new();
        let mut cfg = config();
        cfg.search_domains = vec![domain.into()];
        assert!(h.net.apply(&cfg).is_err());
        assert!(h.commands().is_empty());
    }
}
#[test]
fn hostile_journal_never_executes_an_arbitrary_command() {
    let h = Harness::new();
    std::fs::write(
        &h.journal,
        r#"{"undo":[{"program":"ip","args":["link","del","eth0"]}],"interfaces":["eth0"]}"#,
    )
    .unwrap();
    assert!(h.net.recover().is_err());
    assert!(h.commands().is_empty());
    assert!(h.journal.exists());
}

#[test]
fn journal_apply_command_cannot_hide_behind_a_del_domain() {
    let h = Harness::new();
    std::fs::write(&h.journal, r#"{"undo":[{"program":"resolvectl","args":["domain","glr-wg0","del"]}],"interfaces":["glr-wg0"]}"#).unwrap();
    assert!(h.net.recover().is_err());
    assert!(h.commands().is_empty());
}
#[test]
fn oversized_journal_is_rejected_before_any_command() {
    let h = Harness::new();
    let undo = Cmd {
        program: Program::Resolvectl,
        args: vec!["revert".into(), "glr-wg0".into()],
    };
    let journal = galleria_net::journal::Journal {
        undo: vec![undo; 30_000],
        interfaces: vec!["glr-wg0".into()],
    };
    assert!(journal.write(&h.journal).is_err());
    assert!(!h.journal.exists());
}

#[test]
fn specific_ipv4_routes_stay_in_main_with_a_default_route() {
    let mut cfg = config();
    cfg.routes.push("10.1.0.0/16".parse().unwrap());
    let commands = applied(&cfg);
    assert!(commands.contains(&"ip -4 route add 0.0.0.0/0 dev glr-wg0 table 51820".into()));
    assert!(commands.contains(&"ip -4 route add 10.1.0.0/16 dev glr-wg0".into()));
}
#[test]
fn specific_ipv6_routes_stay_in_main_with_a_default_route() {
    let mut cfg = config();
    cfg.routes = vec![
        "::/0".parse().unwrap(),
        "2001:db8:1234::/48".parse().unwrap(),
    ];
    let commands = applied(&cfg);
    assert!(commands.contains(&"ip -6 route add ::/0 dev glr-wg0 table 51820".into()));
    assert!(commands.contains(&"ip -6 route add 2001:db8:1234::/48 dev glr-wg0".into()));
}
#[test]
fn missing_resolvectl_interface_is_tolerated_during_recovery() {
    struct MissingDns;
    impl CommandRunner for MissingDns {
        fn run(&self, cmd: &Cmd) -> Result<(), NetError> {
            if cmd.program == Program::Resolvectl {
                Err(NetError::Command {
                    cmd: cmd.to_string(),
                    stderr: "Failed to resolve interface glr-ovpn0: No such device".into(),
                })
            } else {
                Ok(())
            }
        }
    }
    let h = Harness::new();
    let mut cfg = config();
    cfg.interface = "glr-ovpn0".into();
    cfg.install_routes = None;
    h.net.apply(&cfg).unwrap();
    let recovered = NetManager::new(MissingDns, h.journal.clone());
    recovered.recover().unwrap();
    assert!(!h.journal.exists());
}
#[derive(Clone)]
struct PartialRunner {
    residual: Arc<Mutex<bool>>,
    fail_undo: bool,
}
impl CommandRunner for PartialRunner {
    fn run(&self, cmd: &Cmd) -> Result<(), NetError> {
        if cmd.to_string() == "ip -6 route replace unreachable 8000::/1" {
            *self.residual.lock().unwrap() = true;
            return Err(NetError::Command {
                cmd: cmd.to_string(),
                stderr: "tool timed out after changing simulated network".into(),
            });
        }
        if cmd.to_string() == "ip -6 route del unreachable 8000::/1" {
            if self.fail_undo {
                return Err(NetError::Command {
                    cmd: cmd.to_string(),
                    stderr: "synthetic undo failure".into(),
                });
            }
            *self.residual.lock().unwrap() = false;
        }
        Ok(())
    }
}
#[test]
fn failed_command_that_already_mutated_network_is_also_rolled_back() {
    let h = Harness::new();
    let residual = Arc::new(Mutex::new(false));
    let net = NetManager::new(
        PartialRunner {
            residual: residual.clone(),
            fail_undo: false,
        },
        h.journal.clone(),
    );
    assert!(net.apply(&config()).is_err());
    assert!(!*residual.lock().unwrap());
    assert!(!h.journal.exists());
}
#[test]
fn failed_command_with_failed_undo_keeps_recovery_journal() {
    let h = Harness::new();
    let residual = Arc::new(Mutex::new(false));
    let net = NetManager::new(
        PartialRunner {
            residual: residual.clone(),
            fail_undo: true,
        },
        h.journal.clone(),
    );
    assert!(net.apply(&config()).is_err());
    assert!(*residual.lock().unwrap());
    assert!(h.journal.exists());
    NetManager::new(
        PartialRunner {
            residual: residual.clone(),
            fail_undo: false,
        },
        h.journal.clone(),
    )
    .recover()
    .unwrap();
    assert!(!*residual.lock().unwrap());
    assert!(!h.journal.exists());
}

#[test]
fn split_defaults_use_policy_but_specific_routes_use_main() {
    let mut cfg = config();
    cfg.routes = [
        "0.0.0.0/1",
        "128.0.0.0/1",
        "10.1.0.0/16",
        "::/1",
        "8000::/1",
        "fd00::/8",
    ]
    .into_iter()
    .map(|s| s.parse().unwrap())
    .collect();
    let commands = applied(&cfg);
    for command in [
        "ip -4 route add 0.0.0.0/1 dev glr-wg0 table 51820",
        "ip -4 route add 128.0.0.0/1 dev glr-wg0 table 51820",
        "ip -6 route add ::/1 dev glr-wg0 table 51820",
        "ip -6 route add 8000::/1 dev glr-wg0 table 51820",
        "ip -4 route add 10.1.0.0/16 dev glr-wg0",
        "ip -6 route add fd00::/8 dev glr-wg0",
    ] {
        assert!(commands.iter().any(|s| s == command), "{command}");
    }
}
#[test]
fn global_ipv6_cover_does_not_move_internal_specific_routes_to_policy_table() {
    let mut cfg = config();
    cfg.routes = ["2000::/3", "2001:db8::/32", "fd00::/8"]
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let commands = applied(&cfg);
    assert!(
        commands
            .iter()
            .any(|s| s == "ip -6 route add 2000::/3 dev glr-wg0 table 51820")
    );
    assert!(
        commands
            .iter()
            .any(|s| s == "ip -6 route add 2001:db8::/32 dev glr-wg0")
    );
    assert!(
        commands
            .iter()
            .any(|s| s == "ip -6 route add fd00::/8 dev glr-wg0")
    );
}
