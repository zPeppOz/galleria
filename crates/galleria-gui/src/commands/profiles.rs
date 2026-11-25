//! Commands of the rubrica: reading and editing. Thin: the work is in `rubrica.rs` and `edit.rs`.

use crate::{
    edit::ProfilePatch,
    error::CmdError,
    export::SavePicker,
    link::DaemonView,
    rubrica::{ProfileDetail, ProfileSummary, RawCheck, Rubrica, RubricaError, RubricaView},
    supervisor::SupervisorHandle,
};
use galleria_ipc::{ProfileRef, State as VpnState};
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
        let (code, data) = match &error {
            RubricaError::Changed => ("gui_profile_changed", None),
            RubricaError::Invalid(fields) => {
                ("gui_invalid_field", Some(json!({ "fields": fields })))
            }
            // The same code and the same `fields` as any refused field, plus the list the editor shows. Never the text.
            RubricaError::InvalidConfig(issues) => (
                "gui_invalid_field",
                Some(json!({
                    "fields": [{ "field": "openvpn_config", "code": "config_issues" }],
                    "issues": issues,
                })),
            ),
            RubricaError::GroupInvalid(reason) => {
                ("gui_group_invalid", Some(json!({ "reason": reason })))
            }
            RubricaError::GroupExists => ("gui_group_exists", None),
            RubricaError::GroupNotFound => ("gui_group_not_found", None),
            RubricaError::InUse => ("gui_profile_in_use", None),
            // The place, so that the window can say where it failed; never the text.
            RubricaError::Export(error) => {
                ("gui_export_failed", Some(json!({ "path": error.path() })))
            }
            RubricaError::Store { dir, source } => match source {
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
                _ => ("gui_store_unreadable", Some(json!({ "path": dir }))),
            },
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

/// Changes a profile and answers with the new detail. `expected_modified` is the time of modification the window
/// last read: a profile written by someone else since then is refused (`gui_profile_changed`), and fields that are
/// not good are refused with their names (`gui_invalid_field`). The patch has no field for a secret or a path.
#[tauri::command(async)]
pub fn profile_update(
    rubrica: State<'_, Arc<Rubrica>>,
    id: Uuid,
    expected_modified: String,
    patch: ProfilePatch,
) -> Result<ProfileDetail, CmdError> {
    Ok(rubrica.update(id, &expected_modified, patch)?)
}

/// What is wrong with the text the editor holds for an OpenVPN profile, line by line, and whether it can be saved. Writes
/// nothing, and the findings carry names of directives, never the text (the save checks again by itself).
#[tauri::command(async)]
pub fn profile_validate_raw(
    rubrica: State<'_, Arc<Rubrica>>,
    id: Uuid,
    text: String,
) -> Result<RawCheck, CmdError> {
    Ok(rubrica.validate_raw(id, &text)?)
}

/// What `profile_export` answers: whether a file was written, and nothing else. The text of the file, the keys in it
/// and the place it went to stay in Rust (Q31).
#[derive(Debug, serde::Serialize)]
pub struct ExportOutcome {
    pub saved: bool,
}

/// The profile of the session the service has open, when there is one that is not `disconnected`: the one that may not
/// be deleted under it.
fn active_profile(view: &DaemonView) -> Option<ProfileRef> {
    view.status
        .as_ref()
        .filter(|status| status.state != VpnState::Disconnected)
        .and_then(|status| status.profile.clone())
}

/// A copy of a profile with the name the window gives, secrets included, made in Rust. Answers with the row of the copy.
/// Works without the daemon.
#[tauri::command(async)]
pub fn profile_duplicate(
    rubrica: State<'_, Arc<Rubrica>>,
    id: Uuid,
    name: String,
) -> Result<ProfileSummary, CmdError> {
    Ok(rubrica.duplicate(id, &name)?)
}

/// Asks the system for a place (a window «Save» that Rust opens) and writes the profile there, keys included. The answer
/// is `{ "saved": bool }`: `false` is a cancel, and neither the text nor the place comes back.
#[tauri::command(async)]
pub fn profile_export(
    rubrica: State<'_, Arc<Rubrica>>,
    picker: State<'_, Arc<dyn SavePicker>>,
    id: Uuid,
) -> Result<ExportOutcome, CmdError> {
    Ok(ExportOutcome {
        saved: rubrica.export(id, &**picker)?,
    })
}

/// Deletes a profile and its secrets, unless it is the one of the session the service has open (`gui_profile_in_use`).
#[tauri::command(async)]
pub fn profile_delete(
    rubrica: State<'_, Arc<Rubrica>>,
    supervisor: State<'_, SupervisorHandle>,
    id: Uuid,
) -> Result<(), CmdError> {
    let active = active_profile(&supervisor.view());
    Ok(rubrica.delete(id, active.as_ref())?)
}
