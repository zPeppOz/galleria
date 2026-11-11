use galleria_ipc::{ErrorCode, SessionError, SessionErrorCode};
use std::time::Duration;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Disconnected,
    Connecting { attempt: u32 },
    NeedsInput { attempt: u32, reconnecting: bool },
    Connected,
    Reconnecting { attempt: u32 },
    Disconnecting { then_connect: bool },
    Error(SessionError),
}
#[derive(Debug, Clone)]
pub enum ExitKind {
    Stopped,
    Auth,
    Transport,
    Fatal(SessionErrorCode),
}
#[derive(Debug)]
pub enum Input {
    Connect { replace: bool },
    Disconnect,
    InputRequired,
    InputProvided,
    InputTimeout,
    Up,
    Restarting,
    NetFailed(SessionErrorCode),
    Exited(ExitKind),
    RetryTimer,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    SpawnBackend,
    StopBackend,
    TeardownNet,
    ForwardInput,
    StartRetryTimer(Duration),
    StartInputTimer(Duration),
    Reject(ErrorCode),
    Publish,
}
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub initial_attempts: u32,
    pub reconnect_attempts: u32,
    pub base: Duration,
    pub cap: Duration,
    pub reconnect_enabled: bool,
}
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_attempts: 4,
            reconnect_attempts: 8,
            base: Duration::from_secs(1),
            cap: Duration::from_secs(30),
            reconnect_enabled: true,
        }
    }
}
pub fn backoff(attempt: u32, policy: &RetryPolicy) -> Duration {
    policy
        .base
        .saturating_mul(
            1_u32
                .checked_shl(attempt.saturating_sub(1))
                .unwrap_or(u32::MAX),
        )
        .min(policy.cap)
}
fn error(code: SessionErrorCode, stop: bool) -> (Phase, Vec<Effect>) {
    let mut effects = vec![Effect::TeardownNet];
    if stop {
        effects.push(Effect::StopBackend);
    }
    effects.push(Effect::Publish);
    (Phase::Error(SessionError::new(code)), effects)
}
pub fn step(phase: &Phase, input: Input, policy: &RetryPolicy) -> (Phase, Vec<Effect>) {
    use Effect::*;
    // Authentication failures never retry, even during a pending replacement.
    if matches!(input, Input::Exited(ExitKind::Auth)) {
        return error(SessionErrorCode::AuthFailed, false);
    }
    if let Input::Exited(ExitKind::Fatal(code)) = input {
        return error(code, false);
    }
    match phase {
        Phase::Disconnected | Phase::Error(_) => match input {
            Input::Connect { .. } => (
                Phase::Connecting { attempt: 1 },
                vec![SpawnBackend, Publish],
            ),
            Input::Disconnect => (Phase::Disconnected, vec![Publish]),
            _ => (phase.clone(), vec![]),
        },
        Phase::Connecting { .. }
        | Phase::NeedsInput { .. }
        | Phase::Connected
        | Phase::Reconnecting { .. }
        | Phase::Disconnecting { .. } => {
            match input {
                Input::Connect { replace: false } => {
                    return (phase.clone(), vec![Reject(ErrorCode::AlreadyActive)]);
                }
                Input::Connect { replace: true } => {
                    return (
                        Phase::Disconnecting { then_connect: true },
                        vec![TeardownNet, StopBackend, Publish],
                    );
                }
                Input::Disconnect => {
                    return (
                        Phase::Disconnecting {
                            then_connect: false,
                        },
                        vec![TeardownNet, StopBackend, Publish],
                    );
                }
                Input::NetFailed(code) => return error(code, true),
                _ => {}
            }
            match phase {
                Phase::Connecting { attempt } | Phase::Reconnecting { attempt } => match input {
                    Input::InputRequired => (
                        Phase::NeedsInput {
                            attempt: *attempt,
                            reconnecting: matches!(phase, Phase::Reconnecting { .. }),
                        },
                        vec![StartInputTimer(Duration::from_secs(120)), Publish],
                    ),
                    Input::Up => (Phase::Connected, vec![Publish]),
                    Input::RetryTimer => (phase.clone(), vec![SpawnBackend]),
                    Input::Exited(ExitKind::Transport) => {
                        let limit = if matches!(phase, Phase::Connecting { .. }) {
                            policy.initial_attempts
                        } else {
                            policy.reconnect_attempts
                        };
                        if *attempt >= limit {
                            return error(SessionErrorCode::ServerUnreachable, false);
                        }
                        let next = if matches!(phase, Phase::Connecting { .. }) {
                            Phase::Connecting {
                                attempt: attempt + 1,
                            }
                        } else {
                            Phase::Reconnecting {
                                attempt: attempt + 1,
                            }
                        };
                        (
                            next,
                            vec![
                                TeardownNet,
                                StartRetryTimer(backoff(*attempt, policy)),
                                Publish,
                            ],
                        )
                    }
                    Input::Exited(_) => error(SessionErrorCode::BackendExited, false),
                    _ => (phase.clone(), vec![]),
                },
                Phase::NeedsInput { attempt, .. } => match input {
                    Input::InputProvided => (
                        Phase::Connecting { attempt: *attempt },
                        vec![ForwardInput, Publish],
                    ),
                    Input::InputTimeout => error(SessionErrorCode::InputTimeout, true),
                    Input::Exited(_) => error(SessionErrorCode::BackendExited, false),
                    _ => (phase.clone(), vec![]),
                },
                Phase::Connected => match input {
                    Input::Restarting => (
                        Phase::Reconnecting { attempt: 1 },
                        vec![TeardownNet, Publish],
                    ),
                    Input::Exited(_) => {
                        if policy.reconnect_enabled {
                            (
                                Phase::Reconnecting { attempt: 1 },
                                vec![TeardownNet, StartRetryTimer(backoff(1, policy)), Publish],
                            )
                        } else {
                            error(SessionErrorCode::BackendExited, false)
                        }
                    }
                    _ => (phase.clone(), vec![]),
                },
                Phase::Disconnecting { then_connect } => match input {
                    Input::Exited(_) => {
                        if *then_connect {
                            (
                                Phase::Connecting { attempt: 1 },
                                vec![SpawnBackend, Publish],
                            )
                        } else {
                            (Phase::Disconnected, vec![Publish])
                        }
                    }
                    _ => (phase.clone(), vec![]),
                },
                Phase::Disconnected | Phase::Error(_) => {
                    unreachable!("handled inactive phases above")
                }
            }
        }
    }
}
