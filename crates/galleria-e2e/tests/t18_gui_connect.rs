#![cfg(target_os = "linux")]
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

/// The `openvpn` that the probe of phase 1 starts (`factory::backend_infos` → `probe`, on `hello`): `<path>/openvpn --version`, and
/// nothing else. It opens no tunnel (Q48: D5 is «never two tunnels together», not «never two processes with the same name»).
fn is_version_probe(process: &str) -> bool {
    // `process` is `<pid> <program> <arguments>`
    let mut words = process.split_whitespace().skip(1);
    let (Some(program), Some(argument), None) = (words.next(), words.next(), words.next()) else {
        return false;
    };
    argument == "--version"
        && program
            .rsplit_once('/')
            .is_some_and(|(_, name)| name == "openvpn")
}

/// An `openvpn` that is exiting: `pgrep -a` shows `<pid> [openvpn]` (or `[openvpn] <defunct>`) for a process whose command line is
/// already gone, that is, one that has let go of its memory and its files. It is the state in which the watch also catches the
/// probe above: 24 times in about 280 samples of `openvpn --version` run in a loop in the image of the client (`pgrep -a` printed
/// `[openvpn]` 18 times and `[openvpn] <defunct>` 6). That is only the text of the line: Q48 leaves such a process out of the
/// backends **only if** `glr-ovpn0` is gone in the same sample too (`ovpn_if=0`); with the interface still there it is a second
/// backend still alive.
fn is_exiting(process: &str) -> bool {
    let words: Vec<&str> = process.split_whitespace().skip(1).collect();
    matches!(words.as_slice(), ["[openvpn]"] | ["[openvpn]", "<defunct>"])
}

/// The backends in a sample: its `openvpn` processes (`<pid> <arguments>`) that can hold a tunnel: not the probe, and not a process
/// that is exiting **while `glr-ovpn0` is gone in that sample** (`ovpn_if` false). Every other one counts, with whatever arguments
/// it has; so does an exiting one when the tunnel interface is still there.
fn openvpn_backends(processes: &[&str], ovpn_if: bool) -> usize {
    processes
        .iter()
        .filter(|p| !is_version_probe(p) && (!is_exiting(p) || ovpn_if))
        .count()
}

/// D5, as the plan has it: not an `openvpn` backend together with `glr-wg0`, nor two `openvpn` backends, in one sample.
fn two_backends_together(openvpn: usize, wg: usize) -> bool {
    (openvpn > 0 && wg > 0) || openvpn > 1
}

struct Sample {
    /// The time of the sample, as the watch wrote it (`date +%s%3N`): what ties it to its `argv` lines.
    ms: String,
    openvpn: usize,
    wg: usize,
    /// `ovpn_if=` of the sample: `glr-ovpn0` was there (in at least one of the two readings of the watch).
    ovpn_if: bool,
}

struct Watch {
    residue: String,
    samples: Vec<Sample>,
    /// `argv <ms> <pid> <arguments>` of every `openvpn` seen, the arguments only.
    argv: Vec<String>,
    /// The same lines whole, with the time of the sample they belong to: `(ms, "<pid> <arguments>")`.
    argv_at: Vec<(String, String)>,
    /// `mark <ms> <name>`: what the window wrote in the register of the watch (`<ms> <name>` in `target/gui-lab/marks`) to say what it
    /// was doing, with the time it wrote it, as `date +%s%3N` would (plan B6, T6, G10g).
    marks: Vec<(u64, String)>,
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
    let mut argv_at = Vec::new();
    let mut marks = Vec::new();
    for line in text.lines() {
        if line == "residue-begin" {
            inside = true;
        } else if line == "residue-end" {
            inside = false;
        } else if inside {
            residue.push_str(line);
            residue.push('\n');
        } else if let Some(rest) = line.strip_prefix("sample ") {
            let mut fields = rest.split_whitespace();
            let ms = fields.next().unwrap_or_default().to_owned();
            let count = |field: Option<&str>, key: &str| -> usize {
                field
                    .and_then(|f| f.strip_prefix(key))
                    .and_then(|n| n.parse().ok())
                    .unwrap_or_else(|| panic!("a sample of the watch is not understood: {line}"))
            };
            let openvpn = count(fields.next(), "openvpn=");
            let wg = count(fields.next(), "wg=");
            // a sample without `ovpn_if=` is a register that is not whole: it does not parse, so the run is red
            let ovpn_if = match count(fields.next(), "ovpn_if=") {
                0 => false,
                1 => true,
                other => {
                    panic!("a sample of the watch is not understood (ovpn_if={other}): {line}")
                }
            };
            samples.push(Sample {
                ms,
                openvpn,
                wg,
                ovpn_if,
            });
        } else if let Some(rest) = line.strip_prefix("argv ") {
            argv.push(rest.splitn(3, ' ').nth(2).unwrap_or_default().to_owned());
            let (ms, process) = rest.split_once(' ').unwrap_or_default();
            argv_at.push((ms.to_owned(), process.to_owned()));
        } else if let Some(rest) = line.strip_prefix("mark ") {
            let (ms, name) = rest
                .split_once(' ')
                .unwrap_or_else(|| panic!("a mark of the watch is not understood: {line}"));
            let ms = ms
                .parse()
                .unwrap_or_else(|_| panic!("a mark of the watch has no time: {line}"));
            marks.push((ms, name.trim().to_owned()));
        }
    }
    Watch {
        residue: residue.trim().to_owned(),
        samples,
        argv,
        argv_at,
        marks,
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
    // D5, as the plan has it: not one sample with an `openvpn` backend and `glr-wg0` together, nor two `openvpn` backends. The watch
    // reads `glr-wg0` twice around the processes (see `lab/tests/gui-lab-watch.sh`), so a tunnel that is going away is not counted;
    // what is left is a fact of one sample, and the message says which one and which processes it saw. The probe of the version
    // (`<path>/openvpn --version`) is not a backend (Q48); an `openvpn` that is exiting (`[openvpn]`) is left out only when
    // `glr-ovpn0` is gone in that same sample (`ovpn_if=0`); nothing else is left out.
    let processes_of = |sample: &Sample| -> Vec<&str> {
        seen.argv_at
            .iter()
            .filter(|(ms, _)| *ms == sample.ms)
            .map(|(_, process)| process.as_str())
            .collect()
    };
    for sample in &seen.samples {
        let processes = processes_of(sample);
        assert_eq!(
            processes.len(),
            sample.openvpn,
            "the sample `sample {} openvpn={} wg={} ovpn_if={}` has {} `argv` lines: the register of the watch is not whole",
            sample.ms,
            sample.openvpn,
            sample.wg,
            u8::from(sample.ovpn_if),
            processes.len()
        );
        if two_backends_together(openvpn_backends(&processes, sample.ovpn_if), sample.wg) {
            let lines: Vec<String> = processes
                .iter()
                .map(|p| format!("argv {} {p}", sample.ms))
                .collect();
            panic!(
                "two backends alive together (an `openvpn` process and glr-wg0, or two `openvpn`), D5, in the sample `sample {} openvpn={} wg={} ovpn_if={}`; the `argv` lines of that instant:\n{}",
                sample.ms,
                sample.openvpn,
                sample.wg,
                u8::from(sample.ovpn_if),
                lines.join("\n")
            );
        }
    }
    assert!(
        seen.samples
            .iter()
            .any(|s| openvpn_backends(&processes_of(s), s.ovpn_if) > 0),
        "the watch never saw an `openvpn` backend (only the probe of the version or a process that was exiting with `glr-ovpn0` gone, if that): the window did not connect an OpenVPN profile"
    );
    assert!(
        seen.samples.iter().any(|s| s.wg == 1),
        "the watch never saw glr-wg0: the window did not connect a WireGuard profile"
    );
}

/// The observer itself (Q48): the probe of the version, and an `openvpn` that is exiting, with a tunnel of the other kind are not two
/// backends; anything else is.
#[test]
#[ignore]
fn the_observer_of_the_backends_leaves_out_only_the_probe_and_the_exiting() {
    let exiting = "4852 [openvpn]";
    assert!(is_exiting(exiting));
    assert!(is_exiting("4852 [openvpn] <defunct>"));
    for not_exiting in [
        "4852 /usr/sbin/openvpn",
        "4852 [openvpn-other]",
        "4852 [openvpn] --config x",
        "4852 /usr/sbin/openvpn --config /dev/stdin",
    ] {
        assert!(!is_exiting(not_exiting), "{not_exiting}");
    }
    // an exiting process with a tunnel of the other kind: green only if `glr-ovpn0` is gone in the sample (`ovpn_if=0`; what round 3 of
    // Q48 saw), red with `ovpn_if=1` (a second backend still alive); with a backend and glr-wg0: red; a backend and an exiting
    // process at once are one backend when `ovpn_if=0` (round 2 of Q48) and two when `ovpn_if=1`; two backends and an exiting
    // process are two
    let backend = "2292 /usr/sbin/openvpn --config /dev/stdin";
    assert!(!two_backends_together(
        openvpn_backends(&[exiting], false),
        1
    ));
    assert!(two_backends_together(openvpn_backends(&[exiting], true), 1));
    assert!(two_backends_together(
        openvpn_backends(&["4852 [openvpn] <defunct>"], true),
        1
    ));
    assert!(!two_backends_together(
        openvpn_backends(&["4852 [openvpn] <defunct>"], false),
        1
    ));
    assert!(!two_backends_together(
        openvpn_backends(&[backend, exiting], false),
        0
    ));
    assert!(two_backends_together(
        openvpn_backends(&[backend, exiting], true),
        0
    ));
    assert!(two_backends_together(
        openvpn_backends(&[backend, exiting], false),
        1
    ));
    assert!(two_backends_together(
        openvpn_backends(&[backend, backend, exiting], false),
        0
    ));
    let probe = "1641 /usr/sbin/openvpn --version";
    let config = "1700 /usr/sbin/openvpn --config /run/galleria/x.ovpn --management 127.0.0.1 7505";
    assert!(is_version_probe(probe));
    assert!(is_version_probe("7 /opt/ovpn/bin/openvpn --version"));
    for not_a_probe in [
        config,
        "1 /usr/sbin/openvpn --version --config x",
        "1 /usr/sbin/openvpn --version=2",
        "1 openvpn --version",
        "1 /usr/sbin/openvpn-other --version",
        "1 /usr/sbin/openvpn",
        "1 /usr/sbin/openvpn --help",
    ] {
        assert!(!is_version_probe(not_a_probe), "{not_a_probe}");
    }
    // the probe with glr-wg0: green; a `--config` with glr-wg0: red
    assert!(!two_backends_together(openvpn_backends(&[probe], false), 1));
    assert!(two_backends_together(openvpn_backends(&[config], false), 1));
    assert!(two_backends_together(
        openvpn_backends(&[probe, config], false),
        1
    ));
    // two backends: red; a backend with the probe: green
    assert!(two_backends_together(
        openvpn_backends(&[config, config], false),
        0
    ));
    assert!(!two_backends_together(
        openvpn_backends(&[probe, config], false),
        0
    ));
    assert!(!two_backends_together(
        openvpn_backends(&[probe, probe], false),
        0
    ));
    assert!(!two_backends_together(0, 1));
}

/// G10g as the client judges it (plan B6, T6): the window wrote three marks in the register, `closed` (it is about to be closed, with the
/// tunnel up), `disconnect` (it is about to ask the tray to disconnect) and `disconnected` (the daemon said so). Between the first two
/// **every** sample must have `glr-wg0` (the margin at the end is half a second: a sample is read a little after its time, and the click
/// of the probe takes longer than that to reach the daemon), and there must be enough of them to be ten seconds; after the second, a
/// sample without `glr-wg0` within five seconds. A mark that is missing is a failure, not a pass: the window did not do it.
fn judge_closed_window(samples: &[Sample], marks: &[(u64, String)]) -> Result<usize, String> {
    let mark = |name: &str| -> Result<u64, String> {
        marks
            .iter()
            .find(|(_, n)| n == name)
            .map(|(ms, _)| *ms)
            .ok_or_else(|| format!("the register of the watch has no mark `{name}`: the window did not get as far as writing it (marks: {marks:?})"))
    };
    let (closed, disconnect, disconnected) =
        (mark("closed")?, mark("disconnect")?, mark("disconnected")?);
    if !(closed < disconnect && disconnect <= disconnected) {
        return Err(format!(
            "the marks are out of order: closed {closed}, disconnect {disconnect}, disconnected {disconnected}"
        ));
    }
    let at = |s: &Sample| -> u64 { s.ms.parse().unwrap_or_default() };
    let kept: Vec<&Sample> = samples
        .iter()
        .filter(|s| at(s) > closed && at(s) + 500 <= disconnect)
        .collect();
    if kept.len() < 30 {
        return Err(format!(
            "{} samples between `closed` and `disconnect` ({} ms apart): want at least 30, the watch was not looking for ten seconds",
            kept.len(),
            disconnect - closed
        ));
    }
    if let Some(gone) = kept.iter().find(|s| s.wg != 1) {
        return Err(format!(
            "glr-wg0 was gone in the sample `sample {} openvpn={} wg={} ovpn_if={}`, {} ms after the window was closed: closing the window took the tunnel down",
            gone.ms,
            gone.openvpn,
            gone.wg,
            u8::from(gone.ovpn_if),
            at(gone) - closed
        ));
    }
    if !samples
        .iter()
        .any(|s| at(s) > disconnect && at(s) <= disconnect + 5000 && s.wg == 0)
    {
        return Err("glr-wg0 was still there 5 s after `disconnect`: the disconnect from the tray did not take the tunnel down".to_owned());
    }
    Ok(kept.len())
}

/// G10, the half that needs the real daemon: the window connected `wg-split`, was closed, and for ten seconds the tunnel of the client
/// stayed up (ui/e2e/lab-connect.e2e.ts, `G10g_the_tunnel_survives_the_closed_window`, wrote the marks).
#[test]
#[ignore]
fn gui_closed_window_kept_the_tunnel() {
    let seen = watch();
    if let Err(why) = judge_closed_window(&seen.samples, &seen.marks) {
        panic!("{why}");
    }
}

/// The judge itself, on made-up registers: the tunnel that stays, the one that goes with the window, a mark that is missing.
#[test]
#[ignore]
fn the_judge_of_the_closed_window_fails_when_the_tunnel_goes_or_a_mark_is_missing() {
    let sample = |ms: u64, wg: usize| Sample {
        ms: ms.to_string(),
        openvpn: 0,
        wg,
        ovpn_if: false,
    };
    let marks = |names: &[(&str, u64)]| -> Vec<(u64, String)> {
        names.iter().map(|(n, ms)| (*ms, (*n).to_owned())).collect()
    };
    let all = marks(&[
        ("closed", 1000),
        ("disconnect", 12_000),
        ("disconnected", 12_900),
    ]);
    // every 200 ms: up until the disconnect, down after it
    let register = |down_at: u64| -> Vec<Sample> {
        (0..80)
            .map(|n| 1000 + n * 200)
            .map(|ms| sample(ms, usize::from(ms < down_at)))
            .collect()
    };
    assert!(judge_closed_window(&register(12_300), &all).is_ok());
    // the tunnel went down two seconds after the window was closed
    let early = judge_closed_window(&register(3000), &all).unwrap_err();
    assert!(
        early.contains("closing the window took the tunnel down"),
        "{early}"
    );
    // the tunnel never went down after the disconnect
    let stuck = judge_closed_window(&register(u64::MAX), &all).unwrap_err();
    assert!(stuck.contains("still there 5 s after"), "{stuck}");
    // a mark that was not written is a failure
    let mut few = all.clone();
    few.retain(|(_, n)| n != "disconnected");
    assert!(
        judge_closed_window(&register(12_300), &few)
            .unwrap_err()
            .contains("no mark `disconnected`")
    );
    assert!(
        judge_closed_window(&register(12_300), &[])
            .unwrap_err()
            .contains("no mark `closed`")
    );
    // too short: the window was closed and the disconnect came at once
    let quick = marks(&[
        ("closed", 1000),
        ("disconnect", 3000),
        ("disconnected", 3500),
    ]);
    assert!(
        judge_closed_window(&register(3200), &quick)
            .unwrap_err()
            .contains("want at least 30")
    );
    // out of order
    let wrong = marks(&[
        ("closed", 5000),
        ("disconnect", 1000),
        ("disconnected", 2000),
    ]);
    assert!(
        judge_closed_window(&register(12_300), &wrong)
            .unwrap_err()
            .contains("out of order")
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
