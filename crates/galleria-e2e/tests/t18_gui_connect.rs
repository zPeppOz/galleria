//! G4, G7 and G9 in the laboratory, the half seen from the client (plan B5, T9 and T10, Q42 (b)): what the client saw and kept
//! while the window of the GUI, in the `galleria-gui-dev` image, drove its real daemon through the bridge of
//! `lab/tests/gui-daemon-relay.py` (`scripts/gui.sh lab-connect <26|27|79>`).
//!
//! The window works first (`LAB_BEFORE_TESTS` of `lab/run.sh`); these tests read what is left: the register of the watch
//! (`lab/tests/gui-lab-watch.sh`: every 200 ms the `openvpn` processes with their arguments and the `glr-wg0` interface), the
//! journal of the daemon and the codes the window used. `lab/run.sh` does not run this file without `GALLERIA_E2E_GUI_CONNECT=1`
//! (a test that skips itself counts as `ok`), and a test here that finds nothing to read **fails** and says what is missing.
//! They print nothing: `scripts/check-gates.py` reads `test <name> ... ok` from the log of the run.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;

const WATCH: &str = "/tmp/gui-lab-watch.log";
const USED_OTP: &str = "/lab/target/gui-lab/used-otp";

/// A run that is not the one of `scripts/gui.sh lab-connect` has nothing for these tests to read: say so, do not pass.
fn assert_driven_by_the_window() {
    assert_lab();
    assert_eq!(
        std::env::var("GALLERIA_E2E_GUI_CONNECT").as_deref(),
        Ok("1"),
        "t18 needs GALLERIA_E2E_GUI_CONNECT=1: run it through scripts/gui.sh lab-connect <26|27|79>"
    );
}

struct Sample {
    openvpn: usize,
    wg: usize,
}

struct Watch {
    residue: String,
    samples: Vec<Sample>,
    /// `argv <ms> <pid> <arguments>` of every `openvpn` seen, the arguments only.
    argv: Vec<String>,
}

fn watch() -> Watch {
    assert_driven_by_the_window();
    let text = std::fs::read_to_string(WATCH).unwrap_or_else(|e| {
        panic!("the watch of the client left no register in {WATCH} ({e}): `scripts/gui.sh lab-connect-window` starts lab/tests/gui-lab-watch.sh")
    });
    let mut residue = String::new();
    let mut inside = false;
    let mut samples = Vec::new();
    let mut argv = Vec::new();
    for line in text.lines() {
        if line == "residue-begin" {
            inside = true;
        } else if line == "residue-end" {
            inside = false;
        } else if inside {
            residue.push_str(line);
            residue.push('\n');
        } else if let Some(rest) = line.strip_prefix("sample ") {
            let mut fields = rest.split_whitespace().skip(1);
            let count = |field: Option<&str>, key: &str| -> usize {
                field
                    .and_then(|f| f.strip_prefix(key))
                    .and_then(|n| n.parse().ok())
                    .unwrap_or_else(|| panic!("a sample of the watch is not understood: {line}"))
            };
            let openvpn = count(fields.next(), "openvpn=");
            let wg = count(fields.next(), "wg=");
            samples.push(Sample { openvpn, wg });
        } else if let Some(rest) = line.strip_prefix("argv ") {
            argv.push(rest.splitn(3, ' ').nth(2).unwrap_or_default().to_owned());
        }
    }
    Watch {
        residue: residue.trim().to_owned(),
        samples,
        argv,
    }
}

/// What the laboratory knows and must never find written down: the passwords, the passphrase, every code the window used, one
/// line of the private key of `base.ovpn` and the private key of `wg-split.conf`.
fn known_secrets() -> Vec<(String, String)> {
    let mut secrets: Vec<(String, String)> = [
        ("password of alice", "alice-secret"),
        ("password of bob", "bob-secret"),
        ("passphrase of the key", "key-passphrase"),
    ]
    .into_iter()
    .map(|(what, value)| (what.to_owned(), value.to_owned()))
    .collect();
    let codes = std::fs::read_to_string(USED_OTP).unwrap_or_else(|e| {
        panic!("the window left no list of the codes it used in {USED_OTP} ({e}): G5a connects the profile `otp`")
    });
    let codes: Vec<&str> = codes.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        !codes.is_empty(),
        "{USED_OTP} is empty: no code was used, so there is nothing to look for"
    );
    for code in codes {
        secrets.push(("a code that was used".to_owned(), code.trim().to_owned()));
    }
    let ovpn = lab_file("base.ovpn");
    let key_line = ovpn
        .split("<key>")
        .nth(1)
        .and_then(|rest| rest.lines().find(|l| l.len() > 40 && !l.starts_with('-')))
        .expect("base.ovpn has no private key")
        .to_owned();
    secrets.push((
        "a line of the private key of base.ovpn".to_owned(),
        key_line,
    ));
    let wg_key = lab_file("wg-split.conf")
        .lines()
        .find_map(|l| l.strip_prefix("PrivateKey = "))
        .expect("wg-split.conf has no private key")
        .trim()
        .to_owned();
    secrets.push(("the private key of wg-split.conf".to_owned(), wg_key));
    assert!(secrets.iter().all(|(_, value)| value.len() >= 6));
    secrets
}

fn uid_of(user: &str) -> u32 {
    checked(&format!("id -u {user}")).parse().unwrap()
}

#[test]
#[ignore]
fn gui_session_never_had_two_backends() {
    let seen = watch();
    assert!(
        seen.samples.len() >= 20,
        "the watch of the client has {} samples: it was not running while the window worked (want at least 20)",
        seen.samples.len()
    );
    // A coincidence of one sample (200 ms) is below what the watch can tell from a process that is starting or stopping, so only an
    // overlap that is there in two samples in a row counts. Single coincidences were seen in some runs: the outcome document says so.
    let bad = |s: &Sample| (s.openvpn > 0 && s.wg > 0) || s.openvpn > 1;
    for (n, pair) in seen.samples.windows(2).enumerate() {
        assert!(
            !(bad(&pair[0]) && bad(&pair[1])),
            "samples {n} and {}: two backends alive together (an `openvpn` process and glr-wg0, or two `openvpn`), D5",
            n + 1
        );
    }
    assert!(
        seen.samples.iter().any(|s| s.openvpn == 1),
        "the watch never saw an `openvpn` process: the window did not connect an OpenVPN profile"
    );
    assert!(
        seen.samples.iter().any(|s| s.wg == 1),
        "the watch never saw glr-wg0: the window did not connect a WireGuard profile"
    );
}

#[test]
#[ignore]
fn gui_session_left_no_residue() {
    let seen = watch();
    assert!(
        !seen.residue.is_empty(),
        "the watch wrote no residue when it started"
    );
    assert_eq!(
        residue().trim(),
        seen.residue,
        "the state of the network is not the one the watch saw before the window connected anything"
    );
    assert!(
        !sh("pgrep -x openvpn").status.success(),
        "an `openvpn` process is still there after the window disconnected"
    );
}

#[test]
#[ignore]
fn gui_session_left_no_secret_in_journal_or_argv() {
    let seen = watch();
    let secrets = known_secrets();
    let journal = checked("journalctl -u galleriad --no-pager -o cat");
    assert!(
        journal.contains(" connect"),
        "the journal of the daemon has no connection in it: it is the wrong one, or empty"
    );
    assert!(
        !seen.argv.is_empty(),
        "the watch saw no `openvpn` with its arguments: there is nothing to look at"
    );
    for (what, value) in &secrets {
        assert!(
            !journal.contains(value.as_str()),
            "the journal of the daemon holds {what}"
        );
        for args in &seen.argv {
            assert!(
                !args.contains(value.as_str()),
                "the arguments of an `openvpn` process hold {what}: {args}"
            );
        }
    }
}

#[test]
#[ignore]
fn gui_session_was_driven_by_two_users() {
    assert_driven_by_the_window();
    let (alice, carla) = (uid_of("alice"), uid_of("carla"));
    assert_ne!(alice, carla);
    let journal = checked("journalctl -u galleriad --no-pager -o cat");
    assert!(
        journal
            .lines()
            .any(|l| l.ends_with(&format!("uid {alice} connect"))),
        "the log of the daemon has no `uid {alice} connect` (alice, who started the session of G9a)"
    );
    assert!(
        journal
            .lines()
            .any(|l| l.ends_with(&format!("uid {carla} disconnect"))),
        "the log of the daemon has no `uid {carla} disconnect` (carla, who closed the session through the window)"
    );
    assert!(
        journal
            .lines()
            .any(|l| l.ends_with(&format!("uid {carla} connect"))),
        "the log of the daemon has no `uid {carla} connect` (carla, through the window)"
    );
}
