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
}

fn watch() -> Watch {
    assert_driven_by_the_window();
    let text = std::fs::read_to_string(WATCH).unwrap_or_else(|e| {
        panic!("the watch of the client left no register in {WATCH} ({e}): `scripts/gui.sh lab-connect-window` starts lab/tests/gui-lab-watch.sh")
    });
    let mut residue = String::new();
    let mut inside = false;
    let mut samples = Vec::new();
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
        }
    }
    Watch {
        residue: residue.trim().to_owned(),
        samples,
    }
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
    for (n, s) in seen.samples.iter().enumerate() {
        assert!(
            !(s.openvpn > 0 && s.wg > 0),
            "sample {n}: an `openvpn` process and glr-wg0 at the same time (D5)"
        );
        assert!(
            s.openvpn <= 1,
            "sample {n}: {} `openvpn` processes at the same time",
            s.openvpn
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
