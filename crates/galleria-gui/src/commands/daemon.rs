//! Commands about the link to the daemon.

use crate::{link::DaemonView, supervisor::SupervisorHandle};
use tauri::State;

/// The last snapshot of the daemon link. No call to the daemon: the supervisor keeps it up to date.
#[tauri::command]
pub fn daemon_view(state: State<'_, SupervisorHandle>) -> DaemonView {
    state.view()
}

/// Makes the supervisor try to reach the daemon now instead of waiting.
#[tauri::command]
pub fn daemon_retry(state: State<'_, SupervisorHandle>) {
    state.retry_now();
}
