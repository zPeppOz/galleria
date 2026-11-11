#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::{ErrorCode, SessionError, SessionErrorCode as E};
use galleriad::machine::*;
use rstest::rstest;
use std::time::Duration;
#[rstest]
#[case(Phase::Disconnected, Input::Connect{replace:false}, Phase::Connecting{attempt:1}, vec![Effect::SpawnBackend,Effect::Publish])]
#[case(Phase::Error(SessionError::new(E::AuthFailed)), Input::Connect{replace:false}, Phase::Connecting{attempt:1}, vec![Effect::SpawnBackend,Effect::Publish])]
#[case(Phase::Connected, Input::Connect{replace:false}, Phase::Connected, vec![Effect::Reject(ErrorCode::AlreadyActive)])]
#[case(Phase::Connected, Input::Connect{replace:true}, Phase::Disconnecting{then_connect:true}, vec![Effect::TeardownNet,Effect::StopBackend,Effect::Publish])]
#[case(Phase::Connecting{attempt:2}, Input::InputRequired, Phase::NeedsInput{attempt:2,reconnecting:false}, vec![Effect::StartInputTimer(Duration::from_secs(120)),Effect::Publish])]
#[case(Phase::Reconnecting{attempt:2}, Input::InputRequired, Phase::NeedsInput{attempt:2,reconnecting:true}, vec![Effect::StartInputTimer(Duration::from_secs(120)),Effect::Publish])]
#[case(Phase::NeedsInput{attempt:2,reconnecting:false}, Input::InputProvided, Phase::Connecting{attempt:2}, vec![Effect::ForwardInput,Effect::Publish])]
#[case(Phase::NeedsInput{attempt:2,reconnecting:true}, Input::InputProvided, Phase::Connecting{attempt:2}, vec![Effect::ForwardInput,Effect::Publish])]
#[case(Phase::NeedsInput{attempt:1,reconnecting:false}, Input::InputTimeout, Phase::Error(SessionError::new(E::InputTimeout)), vec![Effect::TeardownNet,Effect::StopBackend,Effect::Publish])]
#[case(Phase::Connecting{attempt:1}, Input::Up, Phase::Connected, vec![Effect::Publish])]
#[case(Phase::Reconnecting{attempt:1}, Input::Up, Phase::Connected, vec![Effect::Publish])]
#[case(Phase::Connecting{attempt:1}, Input::NetFailed(E::DnsUnavailable), Phase::Error(SessionError::new(E::DnsUnavailable)), vec![Effect::TeardownNet,Effect::StopBackend,Effect::Publish])]
#[case(Phase::Reconnecting{attempt:1}, Input::NetFailed(E::NetSetupFailed), Phase::Error(SessionError::new(E::NetSetupFailed)), vec![Effect::TeardownNet,Effect::StopBackend,Effect::Publish])]
#[case(Phase::Connecting{attempt:1}, Input::Exited(ExitKind::Transport), Phase::Connecting{attempt:2}, vec![Effect::TeardownNet,Effect::StartRetryTimer(Duration::from_secs(1)),Effect::Publish])]
#[case(Phase::Connecting{attempt:4}, Input::Exited(ExitKind::Transport), Phase::Error(SessionError::new(E::ServerUnreachable)), vec![Effect::TeardownNet,Effect::Publish])]
#[case(Phase::Connected, Input::Exited(ExitKind::Transport), Phase::Reconnecting{attempt:1}, vec![Effect::TeardownNet,Effect::StartRetryTimer(Duration::from_secs(1)),Effect::Publish])]
#[case(Phase::Connected, Input::Restarting, Phase::Reconnecting{attempt:1}, vec![Effect::TeardownNet,Effect::Publish])]
#[case(Phase::Reconnecting{attempt:1}, Input::Exited(ExitKind::Transport), Phase::Reconnecting{attempt:2}, vec![Effect::TeardownNet,Effect::StartRetryTimer(Duration::from_secs(1)),Effect::Publish])]
#[case(Phase::Reconnecting{attempt:8}, Input::Exited(ExitKind::Transport), Phase::Error(SessionError::new(E::ServerUnreachable)), vec![Effect::TeardownNet,Effect::Publish])]
#[case(Phase::Connected, Input::Exited(ExitKind::Auth), Phase::Error(SessionError::new(E::AuthFailed)), vec![Effect::TeardownNet,Effect::Publish])]
#[case(Phase::Connected, Input::Disconnect, Phase::Disconnecting{then_connect:false}, vec![Effect::TeardownNet,Effect::StopBackend,Effect::Publish])]
#[case(Phase::Disconnecting{then_connect:false}, Input::Exited(ExitKind::Stopped), Phase::Disconnected, vec![Effect::Publish])]
#[case(Phase::Disconnecting{then_connect:true}, Input::Exited(ExitKind::Stopped), Phase::Connecting{attempt:1}, vec![Effect::SpawnBackend,Effect::Publish])]
fn step_follows_the_spec_table(
    #[case] phase: Phase,
    #[case] input: Input,
    #[case] want: Phase,
    #[case] effects: Vec<Effect>,
) {
    assert_eq!(
        step(&phase, input, &RetryPolicy::default()),
        (want, effects)
    );
}
#[test]
fn backoff_sequence_is_1_2_4_8_16_30_30() {
    assert_eq!(
        (1..=7)
            .map(|n| backoff(n, &RetryPolicy::default()).as_secs())
            .collect::<Vec<_>>(),
        [1, 2, 4, 8, 16, 30, 30]
    );
}
fn active_phases() -> Vec<Phase> {
    vec![
        Phase::Connecting { attempt: 1 },
        Phase::NeedsInput {
            attempt: 1,
            reconnecting: false,
        },
        Phase::Connected,
        Phase::Reconnecting { attempt: 1 },
        Phase::Disconnecting {
            then_connect: false,
        },
    ]
}
#[test]
fn auth_failure_never_retries_from_any_phase() {
    for phase in active_phases() {
        let (next, effects) = step(
            &phase,
            Input::Exited(ExitKind::Auth),
            &RetryPolicy::default(),
        );
        assert_eq!(next, Phase::Error(SessionError::new(E::AuthFailed)));
        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, Effect::StartRetryTimer(_) | Effect::SpawnBackend))
        );
    }
}
#[test]
fn drop_with_reconnect_disabled_goes_to_backend_exited() {
    let policy = RetryPolicy {
        reconnect_enabled: false,
        ..RetryPolicy::default()
    };
    assert_eq!(
        step(
            &Phase::Connected,
            Input::Exited(ExitKind::Transport),
            &policy
        )
        .0,
        Phase::Error(SessionError::new(E::BackendExited))
    );
}
#[test]
fn internal_restart_goes_to_reconnecting_without_respawn() {
    let policy = RetryPolicy::default();
    assert_eq!(
        step(&Phase::Connected, Input::Restarting, &policy),
        (
            Phase::Reconnecting { attempt: 1 },
            vec![Effect::TeardownNet, Effect::Publish]
        )
    );
    assert_eq!(
        step(&Phase::Reconnecting { attempt: 1 }, Input::Up, &policy).0,
        Phase::Connected
    );
    for phase in active_phases()
        .into_iter()
        .filter(|p| *p != Phase::Connected)
    {
        assert_eq!(step(&phase, Input::Restarting, &policy).0, phase);
    }
}
