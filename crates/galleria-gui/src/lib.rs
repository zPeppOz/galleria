#![forbid(unsafe_code)]

//! Rust side of the Galleria GUI: builds the Tauri app that hosts the `ui/` frontend.

pub mod announce;
#[cfg(target_os = "linux")]
pub mod announce_atk;
pub mod commands;
pub mod error;
pub mod events;
pub mod link;
pub mod supervisor;
pub mod ui_state;
pub mod window;

use announce::Announcer;
use commands::app::AppInfo;
use std::{path::PathBuf, sync::Arc};
use supervisor::RelinkPolicy;
use tauri::{Manager, Runtime};
use ui_state::UiStateFile;
use window::WindowSaver;

/// Registers the commands and, at setup, starts the supervisor with a [`events::TauriSink`] and puts it in the app
/// state together with [`AppInfo`]. `state.toml` is read and written at its default path.
pub fn configure<R: Runtime>(builder: tauri::Builder<R>, socket: PathBuf) -> tauri::Builder<R> {
    configure_with(
        builder,
        socket,
        RelinkPolicy::default(),
        UiStateFile::new(UiStateFile::default_path()),
    )
}

/// [`configure`] with the waiting policy and the state file given: the tests use short waits and a temporary file.
pub fn configure_with<R: Runtime>(
    builder: tauri::Builder<R>,
    socket: PathBuf,
    policy: RelinkPolicy,
    state: UiStateFile,
) -> tauri::Builder<R> {
    let state = Arc::new(state);
    let saver = Arc::new(WindowSaver::new(Arc::clone(&state)));
    builder
        .manage(Arc::clone(&state))
        .on_window_event(move |window, event| window::on_event(&saver, window, event))
        .invoke_handler(tauri::generate_handler![
            commands::daemon::daemon_view,
            commands::daemon::daemon_retry,
            commands::app::app_info,
            commands::ui_state::ui_state_get,
            commands::ui_state::ui_state_set,
            commands::announce::announce
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
            // The tests of level 2 manage their own announcer before the build, and `manage` keeps the first one.
            app.manage(default_announcer(app.handle()));
            if let Some(main) = app.get_webview_window("main") {
                window::restore(&main, &state);
            }
            Ok(())
        })
}

/// The announcer of the running app: the ATK signal on Linux, nothing elsewhere.
#[cfg(target_os = "linux")]
fn default_announcer<R: Runtime>(app: &tauri::AppHandle<R>) -> Arc<dyn Announcer> {
    Arc::new(announce_atk::AtkAnnouncer(app.clone()))
}

#[cfg(not(target_os = "linux"))]
fn default_announcer<R: Runtime>(_app: &tauri::AppHandle<R>) -> Arc<dyn Announcer> {
    Arc::new(announce::NoAnnouncer)
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
