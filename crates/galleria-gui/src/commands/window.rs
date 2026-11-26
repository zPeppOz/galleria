//! Commands about the window of the app (plan B6, T2 and T3). Thin: the work is in `window.rs` and `closing.rs`.

use crate::{
    closing::{Closing, Intent},
    window::WindowControl,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

/// The answer of the commands that close: `done` is `false` when the changes that are not saved stopped them (the window is shown and the
/// page was told, `galleria:close-blocked`).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Done {
    pub done: bool,
}

/// Shows the window when it is hidden in the tray. The page calls it when something started from the menu of the tray needs the person (a
/// question, an error, the window of the credentials). No parameter: the window is the main one and the answer is `()`.
#[tauri::command]
pub fn window_show(window: State<'_, Arc<dyn WindowControl>>) {
    window.show();
}

/// The page says whether it holds changes that are not saved: only the bit, never what changed (Q35). Called when it changes value, not at
/// every key, and when the profile that is edited changes.
#[tauri::command]
pub fn unsaved_set(closing: State<'_, Arc<Closing>>, dirty: bool) {
    closing.set_unsaved(dirty);
}

/// Hides the window in the tray, by the same decision as the «X» (`closing::decide`): without a tray that is shown it ends the app, with changes
/// that are not saved it does nothing and says `done: false`. The page calls it after «Discard» or a «Save» that worked.
#[tauri::command]
pub fn window_hide(
    closing: State<'_, Arc<Closing>>,
    window: State<'_, Arc<dyn WindowControl>>,
) -> Done {
    Done {
        done: closing.request(&**window, Intent::Hide),
    }
}

/// Ends the app, by the same decision. The session of the daemon stays (spec §3.4, Q54); with changes that are not saved it does nothing and says
/// `done: false`.
#[tauri::command]
pub fn app_quit(
    closing: State<'_, Arc<Closing>>,
    window: State<'_, Arc<dyn WindowControl>>,
) -> Done {
    Done {
        done: closing.request(&**window, Intent::Quit),
    }
}
