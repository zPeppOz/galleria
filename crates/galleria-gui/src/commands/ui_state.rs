//! Commands about what the window remembers (`state.toml`, D32).

use crate::{
    error::CmdError,
    ui_state::{UiPatch, UiState, UiStateError, UiStateFile},
};
use std::sync::Arc;
use tauri::State;

impl From<UiStateError> for CmdError {
    fn from(error: UiStateError) -> Self {
        Self {
            code: "gui_state_not_saved".to_owned(),
            message: error.to_string(),
            data: None,
        }
    }
}

/// Never fails: a file that is missing or broken gives the defaults.
#[tauri::command]
pub fn ui_state_get(state: State<'_, Arc<UiStateFile>>) -> UiState {
    state.load()
}

/// Changes language, theme, sort order or the version check. The window and the last use are not in the patch: only Rust writes them.
/// A version to dismiss that is not a number of version is refused (`gui_invalid_field`) and nothing is written.
#[tauri::command]
pub fn ui_state_set(
    state: State<'_, Arc<UiStateFile>>,
    patch: UiPatch,
) -> Result<UiState, CmdError> {
    patch.validate().map_err(|field| CmdError {
        code: "gui_invalid_field".to_owned(),
        message: format!("{field} is not a version number"),
        data: Some(serde_json::json!({ "fields": [{ "field": field, "code": "invalid" }] })),
    })?;
    Ok(state.update(|current| patch.apply(current))?)
}
