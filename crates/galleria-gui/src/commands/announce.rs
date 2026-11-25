//! The command that makes an outcome reach the screen reader (Q27).

use crate::{
    announce::{Announcer, Urgency, clean},
    error::CmdError,
};
use std::sync::Arc;
use tauri::State;

/// Cleans the text (the webview is not trusted: it may make Orca say a sentence, nothing else) and hands it to the
/// announcer. Never fails because the signal is missing or the window is gone.
#[tauri::command]
pub fn announce(
    announcer: State<'_, Arc<dyn Announcer>>,
    text: String,
    assertive: bool,
) -> Result<(), CmdError> {
    if let Some(text) = clean(&text) {
        let urgency = if assertive {
            Urgency::Assertive
        } else {
            Urgency::Polite
        };
        announcer.announce(&text, urgency);
    }
    Ok(())
}
