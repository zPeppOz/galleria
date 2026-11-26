//! Commands about the window of the app (plan B6, T2 and T3). Thin: the work is in `window.rs` and `closing.rs`.

use crate::window::WindowControl;
use std::sync::Arc;
use tauri::State;

/// Shows the window when it is hidden in the tray. The page calls it when something started from the menu of the tray needs the person (a
/// question, an error, the window of the credentials). No parameter: the window is the main one and the answer is `()`.
#[tauri::command]
pub fn window_show(window: State<'_, Arc<dyn WindowControl>>) {
    window.show();
}
