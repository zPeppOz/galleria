//! Calls to the daemon that the window makes on its own (spec §3.3): a short connection for each, opened, introduced
//! with `hello`, used once and closed. The long connection of the supervisor only listens. No type of Tauri in here.
//!
//! What goes into a call is a secret, in the case of `connect` and `provide_input`. Nothing here prints, logs or keeps
//! it: the parameters are borrowed or consumed, and the error carries the answer of the daemon, which never has one.

use galleria_ipc::{
    ConnectParams, ConnectResult, ProvideInputParams,
    client::{Client, ClientError},
};
use serde_json::Value;
use std::path::Path;

/// Why a call did not give an answer: the daemon could not be reached, or it answered with an error.
#[derive(Debug, Clone)]
pub enum CallError {
    /// The short connection did not open, or `hello` was refused. The text is for the log of the GUI only.
    Unreachable(String),
    /// The error of the daemon: its code (`already_active`, `not_active`…), the English message and the data it carries.
    Rpc {
        code: String,
        message: String,
        data: Option<Value>,
    },
}

impl CallError {
    fn from_client(error: ClientError) -> Self {
        match error {
            ClientError::Rpc(rpc) => {
                let code = rpc
                    .data
                    .as_ref()
                    .and_then(|data| data.get("code"))
                    .and_then(Value::as_str)
                    .map_or_else(|| "internal".to_owned(), str::to_owned);
                Self::Rpc {
                    code,
                    message: rpc.message,
                    data: rpc.data,
                }
            }
            other => Self::Unreachable(other.to_string()),
        }
    }
}

async fn open(socket: &Path) -> Result<Client, CallError> {
    let mut client = Client::connect(socket)
        .await
        .map_err(CallError::from_client)?;
    client
        .hello("galleria-gui", env!("CARGO_PKG_VERSION"))
        .await
        .map_err(CallError::from_client)?;
    Ok(client)
}

/// Starts a session. The parameters are borrowed: the caller drops (and so wipes) them when this returns.
///
/// # Errors
///
/// [`CallError::Rpc`] with the code of the daemon (`already_active` carries the profile of the active session), or
/// [`CallError::Unreachable`].
pub async fn connect(socket: &Path, params: &ConnectParams) -> Result<ConnectResult, CallError> {
    open(socket)
        .await?
        .call("connect", params)
        .await
        .map_err(CallError::from_client)
}

/// Ends the session, whatever state it is in. `not_active` is the daemon's way to say there was none.
///
/// # Errors
///
/// As [`connect`].
pub async fn disconnect(socket: &Path) -> Result<(), CallError> {
    open(socket)
        .await?
        .call::<_, Value>("disconnect", &serde_json::json!({}))
        .await
        .map(|_| ())
        .map_err(CallError::from_client)
}

/// Gives the daemon what the user typed for a question it asked. The parameters are consumed: they are dropped (and so wiped) when
/// this returns, whatever the outcome, and nothing is kept, logged or put in the error.
///
/// # Errors
///
/// As [`connect`]; `input_not_expected` is the daemon's way to say the question expired or was answered.
pub async fn provide_input(socket: &Path, params: ProvideInputParams) -> Result<(), CallError> {
    let outcome = open(socket)
        .await?
        .call::<_, Value>("provide_input", &params)
        .await
        .map(|_| ())
        .map_err(CallError::from_client);
    drop(params);
    outcome
}
