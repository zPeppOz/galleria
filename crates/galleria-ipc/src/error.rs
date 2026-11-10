use crate::RpcError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    HelloRequired,
    IncompatibleProtocol,
    MessageTooLarge,
    TooManyClients,
    AlreadyActive,
    NotActive,
    InvalidConfig,
    ProfileRejected,
    BackendUnavailable,
    InputNotExpected,
    Internal,
}
impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HelloRequired => "hello_required",
            Self::IncompatibleProtocol => "incompatible_protocol",
            Self::MessageTooLarge => "message_too_large",
            Self::TooManyClients => "too_many_clients",
            Self::AlreadyActive => "already_active",
            Self::NotActive => "not_active",
            Self::InvalidConfig => "invalid_config",
            Self::ProfileRejected => "profile_rejected",
            Self::BackendUnavailable => "backend_unavailable",
            Self::InputNotExpected => "input_not_expected",
            Self::Internal => "internal",
        }
    }
    pub fn to_rpc(self, message: &str) -> RpcError {
        RpcError {
            code: -32000,
            message: message.into(),
            data: Some(serde_json::json!({"code":self.as_str()})),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionErrorCode {
    AuthFailed,
    InputTimeout,
    ServerUnreachable,
    TunnelSetupFailed,
    NetSetupFailed,
    DnsUnavailable,
    UnsupportedChallenge,
    BackendExited,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct SessionError {
    pub code: SessionErrorCode,
    pub message: String,
}
impl SessionError {
    pub fn new(code: SessionErrorCode) -> Self {
        let message = match code {
            SessionErrorCode::AuthFailed => "Authentication failed",
            SessionErrorCode::InputTimeout => "Input timed out",
            SessionErrorCode::ServerUnreachable => "Server unreachable",
            SessionErrorCode::TunnelSetupFailed => "Tunnel setup failed",
            SessionErrorCode::NetSetupFailed => "Network setup failed",
            SessionErrorCode::DnsUnavailable => "DNS unavailable",
            SessionErrorCode::UnsupportedChallenge => "Unsupported challenge",
            SessionErrorCode::BackendExited => "Backend exited",
        };
        Self {
            code,
            message: message.into(),
        }
    }
}
