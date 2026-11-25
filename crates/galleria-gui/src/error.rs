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
