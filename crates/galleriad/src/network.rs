//! Translate network failures at the daemon boundary; error details never reach IPC.
use crate::manager::NetControl;
use galleria_backend::net::NetConfig;
use galleria_ipc::{SessionError, SessionErrorCode};
use galleria_net::{CommandRunner, NetError, NetManager};
pub enum NetOperation {
    Recover,
    Apply(NetConfig),
    Teardown,
}
fn session_error(error: NetError) -> SessionError {
    SessionError::new(match error {
        NetError::DnsUnavailable(_) => SessionErrorCode::DnsUnavailable,
        _ => SessionErrorCode::NetSetupFailed,
    })
}
impl<R: CommandRunner + 'static> NetControl for NetManager<R> {
    fn run(
        &self,
        operation: NetOperation,
        log: &mut dyn FnMut(String),
    ) -> Result<(), SessionError> {
        let result = match operation {
            NetOperation::Recover => NetManager::recover(self),
            NetOperation::Apply(cfg) => NetManager::apply(self, &cfg),
            NetOperation::Teardown => NetManager::teardown(self),
        };
        result.map_err(|error| {
            log(error.to_string());
            session_error(error)
        })
    }
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
