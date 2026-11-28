#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
#[test]
#[ignore]
fn daemon_is_active_under_systemd() {
    assert_eq!(checked("systemctl is-active galleriad"), "active");
}
#[test]
#[ignore]
fn socket_is_root_galleria_0660() {
    assert_eq!(
        checked("stat -c '%U:%G %a' /run/galleria/galleriad.sock"),
        "root:galleria 660"
    );
}
#[tokio::test]
#[ignore]
async fn root_gets_hello() {
    assert_eq!(client().await.hello("test", "1").await.unwrap().protocol, 1);
}
#[test]
#[ignore]
fn group_member_gets_hello() {
    checked("runuser -u alice -- ipc-probe hello");
}
#[test]
#[ignore]
fn non_member_is_denied() {
    assert_eq!(
        sh("runuser -u mallory -- ipc-probe hello").status.code(),
        Some(13)
    );
}
#[test]
#[ignore]
fn unit_hardening_is_active() {
    let s = checked("systemctl show galleriad -p ProtectHome -p NoNewPrivileges");
    assert!(
        s.contains("ProtectHome=yes") && s.contains("NoNewPrivileges=yes"),
        "{s}"
    );
}
#[test]
#[ignore]
fn core_dumps_are_disabled() {
    assert_eq!(
        checked("systemctl show galleriad -p LimitCORE"),
        "LimitCORE=0"
    );
    let limits = checked("cat /proc/$(systemctl show galleriad -p MainPID --value)/limits");
    let line = limits
        .lines()
        .find(|l| l.starts_with("Max core file size"))
        .unwrap();
    let fields: Vec<_> = line.split_whitespace().collect();
    assert_eq!(&fields[4..6], &["0", "0"]);
}
#[tokio::test]
#[ignore]
async fn restart_recovers_and_listens_again() {
    checked("systemctl restart galleriad");
    client().await;
}
#[test]
#[ignore]
fn binfmt_units_are_masked() {
    for unit in [
        "systemd-binfmt.service",
        "proc-sys-fs-binfmt_misc.automount",
    ] {
        assert_eq!(
            String::from_utf8_lossy(&sh(&format!("systemctl is-enabled {unit}")).stdout).trim(),
            "masked"
        );
    }
}
