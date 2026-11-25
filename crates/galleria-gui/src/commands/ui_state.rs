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

/// Changes language, theme or sort order. The window and the last use are not in the patch: only Rust writes them.
#[tauri::command]
pub fn ui_state_set(
    state: State<'_, Arc<UiStateFile>>,
    patch: UiPatch,
) -> Result<UiState, CmdError> {
    Ok(state.update(|current| patch.apply(current))?)
}
