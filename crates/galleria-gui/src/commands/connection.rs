//! Commands of the session: connect a profile, disconnect, and the sentences of the announcements (plan B5, T4). Thin: the
//! work is in `session.rs` and `calls.rs`. What a command answers is `()` or an error; the state of the session reaches the
//! page only with the events (spec §3.3), and no answer has a secret.

use crate::{
    calls::CallError,
    error::CmdError,
    session::{Phrases, SessionFailure, Sessions},
};
use serde_json::json;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

impl From<SessionFailure> for CmdError {
    fn from(failure: SessionFailure) -> Self {
        match failure {
            SessionFailure::Rubrica(error) => error.into(),
            SessionFailure::Call(CallError::Rpc {
                code,
                message,
                data,
            }) => Self {
                code,
                message,
                data,
            },
            SessionFailure::Call(CallError::Unreachable(_)) => Self {
                code: "gui_daemon_unreachable".to_owned(),
                message: "The service cannot be reached".to_owned(),
                data: None,
            },
            // The session of the daemon: whose it is is for the page to ask about, never to replace on its own when it is not ours.
            SessionFailure::AlreadyActive {
                profile,
                foreign,
                message,
            } => Self {
                code: "already_active".to_owned(),
                message,
                data: Some(json!({
                    "profile": { "id": profile.id, "name": profile.name },
                    "foreign": foreign,
                })),
            },
        }
    }
}

/// Connects a profile of the rubrica. The argument is an id and a flag: the keys and the parameters are made in Rust and
/// are never in the page. `replace` is the page asking, after it was told the session is its own or after the user said yes.
#[tauri::command]
pub async fn connect_profile(
    sessions: State<'_, Arc<Sessions>>,
    id: Uuid,
    replace: bool,
) -> Result<(), CmdError> {
    Ok(sessions.connect_profile(id, replace).await?)
}

/// Ends the session of the daemon, whoever started it: the page asks, after a confirmation when it is not its own.
#[tauri::command]
pub async fn disconnect(sessions: State<'_, Arc<Sessions>>) -> Result<(), CmdError> {
    Ok(sessions.disconnect().await?)
}

/// The sentences of the status announcements in the language of the window (Q43). Rust says them when the state changes.
#[tauri::command]
pub fn announce_phrases(
    sessions: State<'_, Arc<Sessions>>,
    phrases: Phrases,
) -> Result<(), CmdError> {
    sessions.set_phrases(phrases);
    Ok(())
}
