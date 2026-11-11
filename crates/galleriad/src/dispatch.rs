use crate::{manager::ManagerHandle, server::DaemonInfo};
use galleria_ipc::*;
use serde_json::{Value, json};
pub async fn dispatch(
    call: Call,
    manager: &ManagerHandle,
    info: &DaemonInfo,
    uid: u32,
) -> Result<Value, RpcError> {
    let result = match call {
        Call::Hello(params) => serde_json::to_value(HelloResult {
            protocol: negotiate(params.protocol)?,
            daemon_version: info.version.clone(),
            backends: info.backends.clone(),
            status: manager.status().await,
        }),
        Call::Status => serde_json::to_value(manager.status().await),
        Call::Connect(params) => serde_json::to_value(manager.connect(params, uid).await?),
        Call::Disconnect => Ok(json!({"status":manager.disconnect(uid).await})),
        Call::ProvideInput(params) => {
            manager.provide_input(params).await?;
            Ok(json!({}))
        }
        // Buffering, audit and subscription filtering are implemented by Task 5.
        Call::Subscribe(_) => Ok(json!({})),
        Call::Logs(_) => return Err(ErrorCode::Internal.to_rpc("Log buffer unavailable")),
    };
    result.map_err(|_| ErrorCode::Internal.to_rpc("Cannot encode response"))
}
