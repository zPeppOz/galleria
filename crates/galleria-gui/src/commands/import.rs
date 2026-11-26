//! Commands of the import: thin, like the others. The text of the profile comes from the page because the user typed or
//! pasted it (spec §3.2, rule 2); nothing else that is a profile does, and no command takes a path (rule 3). Nothing here
//! talks to the daemon: a profile is imported while the service is stopped.

use crate::{
    error::CmdError,
    import::{ImportFailure, ImportPreview, Imports, Origin},
    picker::FilePicker,
    rubrica::ProfileSummary,
};
use serde_json::json;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

impl From<ImportFailure> for CmdError {
    fn from(failure: ImportFailure) -> Self {
        // The message is the English fallback and has no text, no secret and no folder: only what the codes below carry.
        let message = failure.to_string();
        let (code, data) = match failure {
            ImportFailure::Syntax { line } => ("import_syntax", Some(json!({ "line": line }))),
            ImportFailure::MissingFile { file, origin } => (
                "import_missing_file",
                Some(json!({ "file": file, "origin": origin })),
            ),
            ImportFailure::OutsideFolder { file } => {
                ("import_outside_folder", Some(json!({ "file": file })))
            }
            ImportFailure::Invalid => ("import_invalid", None),
            ImportFailure::Unreadable { file_name } => (
                "gui_import_unreadable",
                file_name.map(|name| json!({ "file_name": name })),
            ),
            ImportFailure::TooLarge => ("gui_import_too_large", None),
            ImportFailure::UnsupportedFile { file_name } => (
                "gui_import_unsupported_file",
                file_name.map(|name| json!({ "file_name": name })),
            ),
            ImportFailure::Expired => ("gui_import_expired", None),
            ImportFailure::Rubrica(error) => return CmdError::from(error),
        };
        Self {
            code: code.to_owned(),
            message,
            data,
        }
    }
}

/// The preview of the text the user pasted: the name of the profile, the protocol, the server, the **names** of the secrets
/// found and what was dropped. Nothing is written; the profile waits in memory for `import_commit`.
#[tauri::command(async)]
pub fn import_from_text(
    imports: State<'_, Arc<Imports>>,
    text: String,
) -> Result<ImportPreview, CmdError> {
    Ok(imports.offer_text(&text)?)
}

/// Saves a preview as a profile with the name and the group the window gives. Answers with the row of the new profile.
#[tauri::command(async)]
pub fn import_commit(
    imports: State<'_, Arc<Imports>>,
    id: Uuid,
    name: String,
    group: Option<String>,
) -> Result<ProfileSummary, CmdError> {
    Ok(imports.commit(id, &name, group.as_deref())?)
}

/// Lets a preview go. A preview that is not there any more is not an error.
#[tauri::command(async)]
pub fn import_discard(imports: State<'_, Arc<Imports>>, id: Uuid) {
    imports.discard(id);
}

/// The preview of the file the user chooses in the window «Open» of the system, which Rust opens (the page never sees a
/// path, and the command takes none). `None` is a cancel: the window was closed, and that is not an error.
#[tauri::command(async)]
pub fn import_pick_file(
    imports: State<'_, Arc<Imports>>,
    picker: State<'_, Arc<dyn FilePicker>>,
) -> Result<Option<ImportPreview>, CmdError> {
    let Some(path) = picker.pick() else {
        return Ok(None);
    };
    Ok(Some(imports.offer_path(&path, Origin::Picker)?))
}
