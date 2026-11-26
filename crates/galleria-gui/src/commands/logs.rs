//! The commands of the log (plan B5, T7). Thin: the work is in `logs.rs` and `calls.rs`. `logs_export` takes no argument: the
//! window «Save» is opened here and the place never comes back to the page.

use super::profiles::ExportOutcome;
use crate::{
    calls::CallError,
    error::CmdError,
    export::{ExportError, SavePicker},
    logs,
    session::Sessions,
};
use galleria_ipc::LogsResult;
use serde_json::json;
use std::sync::Arc;
use tauri::State;
use time::OffsetDateTime;

impl From<CallError> for CmdError {
    fn from(error: CallError) -> Self {
        match error {
            CallError::Rpc {
                code,
                message,
                data,
            } => Self {
                code,
                message,
                data,
            },
            CallError::Unreachable(_) => Self {
                code: "gui_daemon_unreachable".to_owned(),
                message: "The service cannot be reached".to_owned(),
                data: None,
            },
        }
    }
}

impl From<ExportError> for CmdError {
    fn from(error: ExportError) -> Self {
        // The place, so that the window can say where it failed; never the text.
        Self {
            code: "gui_export_failed".to_owned(),
            message: "The file could not be written".to_owned(),
            data: Some(json!({ "path": error.path() })),
        }
    }
}

/// A page of the log of the daemon: the lines after `after`, oldest first, at most `limit` (1 to 1000, as the daemon serves).
#[tauri::command]
pub async fn logs_get(
    sessions: State<'_, Arc<Sessions>>,
    after: Option<u64>,
    limit: usize,
) -> Result<LogsResult, CmdError> {
    if limit == 0 || limit > logs::PAGE {
        return Err(CmdError {
            code: "gui_invalid_field".to_owned(),
            message: "The limit is out of range".to_owned(),
            data: Some(json!({ "fields": [{ "field": "limit", "code": "out_of_range" }] })),
        });
    }
    Ok(crate::calls::logs(sessions.socket(), after, limit).await?)
}

/// Reads the whole log again from the daemon (the page does not give it back: what is exported does not pass through the webview),
/// asks the system for a place and writes it there. The answer is `{ "saved": bool }`: `false` is a cancel.
#[tauri::command]
pub async fn logs_export(
    sessions: State<'_, Arc<Sessions>>,
    picker: State<'_, Arc<dyn SavePicker>>,
) -> Result<ExportOutcome, CmdError> {
    let entries = logs::all(sessions.socket()).await?;
    let picker = Arc::clone(&picker);
    // the window «Save» waits for a person: not on a thread of the async runtime that others need
    let saved = tauri::async_runtime::spawn_blocking(move || {
        logs::save(&entries, &*picker, OffsetDateTime::now_utc())
    })
    .await
    .map_err(|_| CmdError {
        code: "internal".to_owned(),
        message: "The task ended".to_owned(),
        data: None,
    })??;
    Ok(ExportOutcome { saved })
}
