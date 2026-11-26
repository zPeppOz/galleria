//! G3 in the laboratory (plan B4, T6, Q37 (a)): the profiles the window of the GUI imported are read and connected by the
//! real CLI and the real daemon on the client. The window itself runs in the `galleria-gui-dev` image (`scripts/gui.sh
//! lab-import`), which writes its folder of profiles to `target/gui-lab/config/galleria`; the client sees the repository
//! at `/lab`. Without `GALLERIA_E2E_GUI_STORE=1` every test here skips and says so, as the DCO test does.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
use galleria_ipc::*;
use std::{
    io::Write,
    process::{Child, Command, Output, Stdio},
    time::Duration,
};

const STORE: &str = "/lab/target/gui-lab/config/galleria";

fn wanted() -> bool {
    let on = std::env::var("GALLERIA_E2E_GUI_STORE").as_deref() == Ok("1");
    if !on {
        eprintln!("GUI store skipped: requires GALLERIA_E2E_GUI_STORE=1 (scripts/gui.sh lab-g3)");
    }
    on
}

fn spawn_as_alice(args: &[&str]) -> Child {
    assert_lab();
    Command::new("runuser")
        .args(["-u", "alice", "--", "/usr/local/bin/galleria"])
        .args(args)
        .env("LC_ALL", "C")
        .env("XDG_CONFIG_HOME", "/home/alice/.config")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn cli(args: &[&str], input: &[u8]) -> Output {
    let mut child = spawn_as_alice(args);
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn success(args: &[&str], input: &[u8]) -> String {
    let out = cli(args, input);
    assert!(
        out.status.success(),
        "CLI failed ({:?}): {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// What the window wrote, as it is the moment the folder leaves the image: copied to `alice` with the modes of a real
/// store (`0700` folders, `0600` files), and nothing else of hers left from another test.
async fn with_the_store_of_the_window() -> client::Client {
    let mut c = client().await;
    disconnect(&mut c).await;
    assert!(
        std::path::Path::new(STORE).join("profiles").is_dir(),
        "the window left no folder of profiles in {STORE}: run `scripts/gui.sh lab-import` first"
    );
    checked(&format!(
        "rm -rf /home/alice/.config/galleria; install -d -o alice -g alice -m700 /home/alice/.config; \
         cp -r '{STORE}' /home/alice/.config/galleria; chown -R alice:alice /home/alice/.config; \
         find /home/alice/.config/galleria -type d -exec chmod 700 {{}} +; \
         find /home/alice/.config/galleria -type f -exec chmod 600 {{}} +"
    ));
    c
}

fn no_script_ran() -> String {
    // The unit of the daemon has `PrivateTmp=yes`: what a script of OpenVPN would write is in the private `/tmp` of the
    // daemon, not in this one, so both are looked at.
    let pid = checked("systemctl show galleriad -p MainPID --value");
    checked(&format!(
        "{{ find /tmp -name 'PWNED-*'; if [ '{pid}' != 0 ]; then find /proc/{pid}/root/tmp -name 'PWNED-*' 2>/dev/null; fi; }} | sort"
    ))
}

#[tokio::test]
#[ignore]
async fn gui_imported_profiles_are_listed_by_the_cli() {
    if !wanted() {
        return;
    }
    with_the_store_of_the_window().await;
    let listed = success(&["list"], &[]);
    for name in ["base", "malicious", "wg-split"] {
        assert!(
            listed
                .lines()
                .any(|l| l.split_whitespace().any(|w| w == name)),
            "the CLI does not list «{name}»: {listed}"
        );
    }
    assert_eq!(
        listed.lines().filter(|l| !l.trim().is_empty()).count(),
        3,
        "{listed}"
    );
}

#[tokio::test]
#[ignore]
async fn gui_imported_wireguard_connects_from_the_cli() {
    if !wanted() {
        return;
    }
    let mut c = with_the_store_of_the_window().await;
    let before = residue();
    success(&["connect", "wg-split"], &[]);
    assert_eq!(success(&["status"], &[]).trim(), "connected");
    checked("ping -c 2 -W 2 10.99.0.1");
    success(&["disconnect"], &[]);
    wait_state(&mut c, State::Disconnected, Duration::from_secs(5)).await;
    assert_eq!(residue(), before);
}

#[tokio::test]
#[ignore]
async fn gui_imported_openvpn_connects_from_the_cli() {
    if !wanted() {
        return;
    }
    let mut c = with_the_store_of_the_window().await;
    let before = residue();
    success(
        &["connect", "base", "--username", "alice", "--password-stdin"],
        b"alice-secret\n",
    );
    let s: Status = serde_json::from_str(&success(&["status", "--json"], &[])).unwrap();
    assert_eq!(s.state, State::Connected);
    checked("ping -c 2 -W 2 10.8.0.1");
    disconnect(&mut c).await;
    assert_eq!(residue(), before);
}

#[tokio::test]
#[ignore]
async fn gui_imported_malicious_profile_runs_no_script() {
    if !wanted() {
        return;
    }
    let mut c = with_the_store_of_the_window().await;
    let before = residue();
    assert_eq!(
        no_script_ran(),
        "",
        "a PWNED file was there before the connection"
    );
    // what the window saved has none of the lines it took out
    let saved = checked("cat /home/alice/.config/galleria/profiles/*.toml");
    assert!(
        !saved.contains("PWNED"),
        "a line of the script is in a saved profile"
    );
    // What the daemon answers is recorded as it is: with the script directives dropped by the import what is left is
    // the profile `base`, so `connected` is what is expected; if the daemon refuses, the test says what it said.
    let out = cli(
        &[
            "connect",
            "malicious",
            "--username",
            "alice",
            "--password-stdin",
        ],
        b"alice-secret\n",
    );
    let said = format!(
        "exit {:?}; stdout «{}»; stderr «{}»",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).trim(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    println!("connect of the imported malicious profile: {said}");
    let state: Status = serde_json::from_str(&success(&["status", "--json"], &[])).unwrap();
    println!("state after the connection: {:?}", state.state);
    // the processes of OpenVPN, while the tunnel is up: none has a script, a plugin or a management on a network address
    let processes = checked("ps -eo args | grep -E '(^|/)openvpn( |$)' | grep -v grep || true");
    println!("openvpn processes:\n{processes}");
    for forbidden in [
        "--up",
        "--plugin",
        "--management 0.0.0.0",
        "--script-security",
    ] {
        assert!(
            !processes.contains(forbidden),
            "openvpn runs with {forbidden}: {processes}"
        );
    }
    // nothing a script of the profile would have made, in either `/tmp`
    let found = no_script_ran();
    println!("PWNED files: {found:?}");
    assert_eq!(found, "", "a script of the profile ran");
    // the control: the same look does see a file of that name, in the `/tmp` of the test and in the private one of the daemon
    let pid = checked("systemctl show galleriad -p MainPID --value");
    checked(&format!(
        "touch /tmp/PWNED-control && nsenter -t {pid} -m -- touch /tmp/PWNED-control"
    ));
    let seen = no_script_ran();
    checked(&format!(
        "rm -f /tmp/PWNED-control && nsenter -t {pid} -m -- rm -f /tmp/PWNED-control"
    ));
    assert!(
        seen.lines().any(|l| l == "/tmp/PWNED-control")
            && seen
                .lines()
                .any(|l| l.starts_with("/proc/") && l.ends_with("/root/tmp/PWNED-control")),
        "the look for PWNED files cannot see them: {seen:?}"
    );
    assert_eq!(no_script_ran(), "", "the control file was not removed");
    assert!(
        out.status.success(),
        "the daemon refused the cleaned profile: {said}"
    );
    assert_eq!(state.state, State::Connected, "{said}");
    disconnect(&mut c).await;
    assert_eq!(residue(), before, "the connection left something behind");
    assert_eq!(no_script_ran(), "", "a script ran at the disconnection");
}
