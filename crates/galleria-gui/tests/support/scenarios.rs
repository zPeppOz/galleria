//! The scripts of the fake backend that the window is tried against (plan B5, P16): one list per name, one script for each
//! `connect` the daemon gets. A list that runs out leaves the next connection with a backend that says nothing, so every
//! list is long enough for a whole test. Used by the tests of level 2 and by the fake daemon of the smoke test
//! (`examples/fake-daemon.rs --scenario <name>`).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use galleria_backend::{fake::*, *};
use galleria_ipc::{InputKind, InputRequest};
use std::time::Duration;

/// Every name `scenario` knows.
pub const NAMES: &[&str] = &[
    "plain",
    "credentials",
    "otp",
    "passphrase",
    "auth-failed",
    "unreachable",
    "stats",
    "slow",
];

const TURNS: usize = 12;

fn ask(kind: InputKind, prompt: &str, echo: bool) -> FakeStep {
    FakeStep::Emit(FakeEvent::InputRequired(InputRequest {
        request: 1,
        kind,
        prompt: prompt.into(),
        echo,
        expires_in_secs: 120,
    }))
}

fn tunnel() -> Vec<FakeStep> {
    vec![
        FakeStep::Emit(FakeEvent::NetUp(super::config("10.0.0.1"))),
        super::up(),
    ]
}

fn plain() -> Vec<FakeStep> {
    let mut steps = vec![FakeStep::Emit(FakeEvent::Progress("WAIT".into()))];
    steps.extend(tunnel());
    steps
}

fn asking(kind: InputKind, prompt: &str, echo: bool) -> Vec<FakeStep> {
    let mut steps = vec![ask(kind, prompt, echo), FakeStep::WaitCommand];
    steps.extend(tunnel());
    steps
}

fn refused() -> Vec<FakeStep> {
    vec![
        ask(InputKind::Credentials, "Username and password", false),
        FakeStep::WaitCommand,
        FakeStep::Emit(FakeEvent::Exited(ExitReason::AuthFailed(
            "Authentication failed".into(),
        ))),
    ]
}

fn unreachable() -> Vec<FakeStep> {
    vec![FakeStep::Emit(FakeEvent::Exited(ExitReason::Fatal(
        galleria_ipc::SessionError::new(galleria_ipc::SessionErrorCode::ServerUnreachable),
    )))]
}

/// A tunnel whose counters grow every 200 ms for about two minutes.
fn counting() -> Vec<FakeStep> {
    let mut steps = tunnel();
    for n in 1..=600_u64 {
        steps.push(FakeStep::Emit(FakeEvent::Stats(galleria_ipc::Stats {
            rx_bytes: n * 2048,
            tx_bytes: n * 512,
            last_handshake: Some("1795000000".into()),
        })));
        steps.push(FakeStep::Sleep(Duration::from_millis(200)));
    }
    steps
}

fn slow() -> Vec<FakeStep> {
    vec![
        FakeStep::Emit(FakeEvent::Progress("TCP_CONNECT".into())),
        FakeStep::Sleep(Duration::from_secs(3600)),
    ]
}

/// The scripts of a scenario, one for each connection; `None` for a name that is not one.
pub fn scenario(name: &str) -> Option<Vec<Vec<FakeStep>>> {
    let repeat = |make: &dyn Fn() -> Vec<FakeStep>| (0..TURNS).map(|_| make()).collect();
    Some(match name {
        "plain" => repeat(&plain),
        // the first connection asks for the user and the password, the next ones (their profile remembers them) do not
        "credentials" => {
            let mut scripts = vec![asking(
                InputKind::Credentials,
                "Username and password",
                false,
            )];
            scripts.extend((0..TURNS).map(|_| plain()));
            scripts
        }
        "otp" => repeat(&|| asking(InputKind::Otp, "Enter OTP", true)),
        "passphrase" => repeat(&|| asking(InputKind::KeyPassphrase, "Enter the passphrase", false)),
        "auth-failed" => repeat(&refused),
        "unreachable" => repeat(&unreachable),
        "stats" => repeat(&counting),
        "slow" => repeat(&slow),
        _ => return None,
    })
}
