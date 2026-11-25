//! Commands of the rubrica in reading. Thin: the work is in `rubrica.rs`.

use crate::{
    error::CmdError,
    rubrica::{ProfileDetail, Rubrica, RubricaError, RubricaView},
};
use galleria_profile::StoreError;
use serde_json::json;
use std::{path::Path, sync::Arc};
use tauri::State;
use uuid::Uuid;

/// The command the user can copy to fix a path whose permissions are too open: `chmod 700` for a folder, `chmod 600`
/// for a file, with the path between single quotes.
fn chmod_command(path: &Path) -> String {
    let mode = if path.is_dir() { "700" } else { "600" };
    format!("chmod {mode} {}", shell_quote(path))
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

impl From<RubricaError> for CmdError {
    fn from(error: RubricaError) -> Self {
        let message = error.to_string();
        let (code, data) = match &error.source {
            StoreError::NotFound => ("gui_profile_not_found", None),
            StoreError::InsecurePermissions { path, mode } => (
                "gui_store_insecure",
                Some(json!({
                    "path": path,
                    "mode": format!("{mode:o}"),
                    "fix": chmod_command(path),
                })),
            ),
            StoreError::WrongOwner { path } => (
                "gui_store_insecure",
                Some(json!({
                    "path": path,
                    "fix": format!("sudo chown -R \"$USER\" {}", shell_quote(path)),
                })),
            ),
            // The folder is in the data so that the window can say where the problem is; never the content of a file.
            _ => ("gui_store_unreadable", Some(json!({ "path": error.dir }))),
        };
        Self {
            code: code.to_owned(),
            message,
            data,
        }
    }
}

/// Every profile, the groups in their order, and the files that were skipped. Works without the daemon.
#[tauri::command(async)]
pub fn profiles_list(rubrica: State<'_, Arc<Rubrica>>) -> Result<RubricaView, CmdError> {
    Ok(rubrica.list()?)
}

/// One profile, without its secrets. The argument is an id and nothing else: no name and no path (spec §3.2, rule 3).
#[tauri::command(async)]
pub fn profile_get(rubrica: State<'_, Arc<Rubrica>>, id: Uuid) -> Result<ProfileDetail, CmdError> {
    Ok(rubrica.get(id)?)
}
