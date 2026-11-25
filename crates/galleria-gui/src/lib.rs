#![forbid(unsafe_code)]

//! Rust side of the Galleria GUI: builds the Tauri app that hosts the `ui/` frontend.

pub mod commands;
pub mod error;
pub mod events;
pub mod link;
pub mod supervisor;

use commands::app::AppInfo;
use std::path::PathBuf;
use supervisor::RelinkPolicy;
use tauri::{Manager, Runtime};

/// Registers the three commands and, at setup, starts the supervisor with a [`events::TauriSink`] and puts it in
/// the app state together with [`AppInfo`].
pub fn configure<R: Runtime>(builder: tauri::Builder<R>, socket: PathBuf) -> tauri::Builder<R> {
    configure_with(builder, socket, RelinkPolicy::default())
}

/// [`configure`] with the waiting policy given: the tests use short waits.
pub fn configure_with<R: Runtime>(
    builder: tauri::Builder<R>,
    socket: PathBuf,
    policy: RelinkPolicy,
) -> tauri::Builder<R> {
    builder
        .invoke_handler(tauri::generate_handler![
            commands::daemon::daemon_view,
            commands::daemon::daemon_retry,
            commands::app::app_info
        ])
        .setup(move |app| {
            // The supervisor runs on the async runtime of Tauri; setup itself is not inside it.
            let _in_runtime = tauri::async_runtime::handle().inner().enter();
            let sink = std::sync::Arc::new(events::TauriSink(app.handle().clone()));
            let handle = supervisor::spawn(
                socket.clone(),
                env!("CARGO_PKG_VERSION").to_owned(),
                sink,
                policy,
            );
            app.manage(handle);
            app.manage(AppInfo::new(&socket));
            Ok(())
        })
}

/// Builds the Tauri application from `tauri.conf.json` and runs it until the last window closes. The daemon
/// socket is `GALLERIA_SOCKET` when set, the default path otherwise.
///
/// # Errors
///
/// Returns the Tauri error when the application cannot be built or crashes while running.
pub fn run() -> tauri::Result<()> {
    let socket = link::socket_path(std::env::var_os("GALLERIA_SOCKET"));
    configure(tauri::Builder::default(), socket).run(tauri::generate_context!())
}
