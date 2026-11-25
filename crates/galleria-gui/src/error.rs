//! The error every command returns to the webview (spec §4): a stable code and an English fallback message.

use serde::Serialize;

/// `code` is stable and translated by the webview (D22); `message` is the English fallback. Daemon codes pass
/// through unchanged, the GUI's own start with `gui_`.
#[derive(Debug, Clone, Serialize)]
pub struct CmdError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Every code a command of the GUI can return, apart from the daemon's own: those that start with `gui_` and, from
/// B4, those that start with `import_`. A code is added here in the same commit that introduces it, together with
/// its text in both catalogues of the webview; `tests/catalogs.rs` fails when either is missing.
pub const GUI_CODES: &[&str] = &[
    "gui_state_not_saved",
    "gui_store_insecure",
    "gui_store_unreadable",
    "gui_profile_not_found",
    "gui_profile_newer",
    "gui_profile_invalid",
    "gui_profile_changed",
    "gui_invalid_field",
    "gui_group_invalid",
    "gui_group_exists",
    "gui_group_not_found",
    "gui_profile_in_use",
    "gui_export_failed",
    "import_syntax",
    "import_missing_file",
    "import_outside_folder",
    "import_invalid",
    "gui_import_unreadable",
    "gui_import_too_large",
    "gui_import_unsupported_file",
    "gui_import_expired",
];
