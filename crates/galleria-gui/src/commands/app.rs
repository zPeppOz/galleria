//! Commands about the app itself.

use serde::Serialize;
use std::path::Path;
use tauri::State;

/// What the first-run panels need to build the commands the user copies (D27).
#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub socket_path: String,
    pub user: String,
    pub group: String,
}

impl AppInfo {
    /// `user` comes from the system account of the running process, not from `$USER`.
    pub fn new(socket: &Path) -> Self {
        let uid = nix::unistd::getuid();
        let user = match nix::unistd::User::from_uid(uid) {
            Ok(Some(user)) => user.name,
            _ => uid.to_string(),
        };
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            socket_path: socket.display().to_string(),
            user,
            group: "galleria".to_owned(),
        }
    }
}

#[tauri::command]
pub fn app_info(state: State<'_, AppInfo>) -> AppInfo {
    state.inner().clone()
}
