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
    "stuck",
    "chatty",
    "leaky",
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

/// Asks for the user and the password, takes the answer and then never gets to `connected`: what an answer that has been sent
/// and not yet accepted looks like, for the credentials that are held until the session is up.
fn stuck() -> Vec<FakeStep> {
    vec![
        ask(InputKind::Credentials, "Username and password", false),
        FakeStep::WaitCommand,
        FakeStep::Sleep(Duration::from_secs(3600)),
    ]
}

/// A session that writes more lines than the buffer of the daemon holds (2000): the log has to be read in several pages.
fn chatty() -> Vec<FakeStep> {
    let mut steps: Vec<FakeStep> = (1..=2100)
        .map(|n| {
            FakeStep::Emit(FakeEvent::Log(LogLine {
                level: galleria_ipc::LogLevel::Info,
                message: format!("line {n}"),
            }))
        })
        .collect();
    steps.extend(tunnel());
    steps
}

/// A backend that repeats the keys it was given in its own log, as a careless one would: the daemon has to redact them
/// (the names of the profile `Homelab` of the seed carry `SEED-SECRET-`).
fn leaky() -> Vec<FakeStep> {
    let mut steps: Vec<FakeStep> = ["SEED-SECRET-homelab-private", "SEED-SECRET-homelab-psk"]
        .into_iter()
        .map(|key| {
            FakeStep::Emit(FakeEvent::Log(LogLine {
                level: galleria_ipc::LogLevel::Warn,
                message: format!("handshake with key {key} done"),
            }))
        })
        .collect();
    steps.extend(tunnel());
    steps
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
        "stuck" => repeat(&stuck),
        "chatty" => repeat(&chatty),
        "leaky" => repeat(&leaky),
        _ => return None,
    })
}

/// What the fake daemon says it has in `hello`: both backends, and the fake backend's own capabilities (a handshake to show).
pub fn backends() -> Vec<galleria_ipc::BackendInfo> {
    use galleria_ipc::{BackendInfo, Capabilities, Protocol};
    let capabilities = Capabilities {
        handshake: true,
        interactive_auth: true,
        server_pushed_config: true,
    };
    [Protocol::Wireguard, Protocol::Openvpn]
        .into_iter()
        .map(|protocol| BackendInfo {
            protocol,
            available: true,
            version: Some("fake".into()),
            unavailable_reason: None,
            capabilities: Capabilities {
                handshake: protocol == Protocol::Wireguard,
                ..capabilities
            },
        })
        .collect()
}
