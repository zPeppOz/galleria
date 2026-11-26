//! The command that gives Rust the texts of the tray (plan B6, T1, P3).

use crate::{
    error::CmdError,
    rubrica::Rubrica,
    tray::{Tray, TrayLabels},
    tray_tauri::favourites_of,
};
use serde_json::json;
use std::sync::Arc;
use tauri::State;

/// The texts of the menu of the tray, in the language of the window: at the start and at every change of language. The first
/// time makes the tray. A text that is empty or longer than 200 characters is refused (`gui_invalid_field`): the page is not
/// trusted with what the desktop draws. Not `async`: the tray belongs to the thread of the window, which runs this command.
#[tauri::command]
pub fn tray_labels(
    tray: State<'_, Arc<Tray>>,
    rubrica: State<'_, Arc<Rubrica>>,
    labels: TrayLabels,
) -> Result<(), CmdError> {
    let labels = labels.checked().map_err(|_| CmdError {
        code: "gui_invalid_field".to_owned(),
        message: "A text of the tray is empty or too long".to_owned(),
        data: Some(json!({ "fields": [{ "field": "labels", "code": "invalid" }] })),
    })?;
    tray.set_favourites(favourites_of(&rubrica));
    tray.set_labels(labels);
    Ok(())
}
