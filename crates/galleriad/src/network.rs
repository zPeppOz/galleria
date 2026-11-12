//! Translate network failures at the daemon boundary; error details never reach IPC.
use crate::manager::NetControl;
use galleria_backend::net::NetConfig;
use galleria_ipc::{SessionError, SessionErrorCode};
use galleria_net::{CommandRunner, NetError, NetManager};
fn session_error(error: NetError) -> SessionError {
    let mut result = SessionError::new(match &error {
        NetError::DnsUnavailable(_) => SessionErrorCode::DnsUnavailable,
        _ => SessionErrorCode::NetSetupFailed,
    });
    result.message = error.to_string();
    result
}
impl<R: CommandRunner + 'static> NetControl for NetManager<R> {
    fn recover(&self) -> Result<(), SessionError> {
        NetManager::recover(self).map_err(session_error)
    }
    fn apply(&self, cfg: &NetConfig) -> Result<(), SessionError> {
        NetManager::apply(self, cfg).map_err(session_error)
    }
    fn teardown(&self) -> Result<(), SessionError> {
        NetManager::teardown(self).map_err(session_error)
    }
}
