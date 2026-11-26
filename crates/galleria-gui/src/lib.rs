#![forbid(unsafe_code)]

//! Rust side of the Galleria GUI: builds the Tauri app that hosts the `ui/` frontend.

pub mod announce;
#[cfg(target_os = "linux")]
pub mod announce_atk;
pub mod calls;
pub mod commands;
pub mod edit;
pub mod error;
pub mod events;
pub mod export;
#[cfg(target_os = "linux")]
pub mod export_gtk;
pub mod import;
pub mod instance;
pub mod link;
pub mod picker;
#[cfg(target_os = "linux")]
pub mod picker_gtk;
pub mod rubrica;
pub mod session;
pub mod supervisor;
pub mod ui_state;
pub mod window;

use announce::Announcer;
use commands::app::AppInfo;
use export::SavePicker;
use import::Imports;
use picker::FilePicker;
use rubrica::Rubrica;
use session::Sessions;
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
        .on_window_event(move |window, event| {
            window::on_event(&saver, window, event);
            events::on_drag_drop(window, event);
        })
        .invoke_handler(tauri::generate_handler![
            commands::daemon::daemon_view,
            commands::daemon::daemon_retry,
            commands::app::app_info,
            commands::ui_state::ui_state_get,
            commands::ui_state::ui_state_set,
            commands::announce::announce,
            commands::profiles::profiles_list,
            commands::profiles::profile_get,
            commands::profiles::profile_update,
            commands::profiles::profile_validate_raw,
            commands::profiles::profile_duplicate,
            commands::profiles::profile_export,
            commands::profiles::profile_delete,
            commands::groups::group_create,
            commands::groups::group_rename,
            commands::groups::group_delete,
            commands::groups::groups_reorder,
            commands::import::import_from_text,
            commands::import::import_commit,
            commands::import::import_discard,
            commands::import::import_pick_file,
            commands::import::import_pending,
            commands::secrets::secret_store_status,
            commands::connection::connect_profile,
            commands::connection::disconnect,
            commands::connection::announce_phrases,
            commands::connection::provide_input,
            commands::connection::forget_credentials
        ])
        .setup(move |app| {
            // The supervisor runs on the async runtime of Tauri; setup itself is not inside it.
            let _in_runtime = tauri::async_runtime::handle().inner().enter();
            // Everything the events meet on their way (the rubrica, the session, the announcer) is in the state before the
            // supervisor starts. The tests of level 2 manage their own rubrica (in a temporary folder) and their own announcer
            // before the build, and `manage` keeps the first one.
            app.manage(Arc::new(Rubrica::new(
                Rubrica::default_dir(),
                Arc::clone(&state),
            )));
            app.manage(default_announcer(app.handle()));
            let rubrica = Arc::clone(&*app.state::<Arc<Rubrica>>());
            app.manage(Arc::new(Sessions::new(
                socket.clone(),
                Arc::clone(&rubrica),
                Arc::clone(&state),
            )));
            let sink = std::sync::Arc::new(events::TauriSink(app.handle().clone()));
            let handle = supervisor::spawn(
                socket.clone(),
                env!("CARGO_PKG_VERSION").to_owned(),
                sink,
                policy,
            );
            app.manage(handle);
            app.manage(AppInfo::new(&socket));
            // The previews of the import wait over the rubrica that is in the state (the one of the test, when there is one).
            app.manage(Arc::new(Imports::new(rubrica)));
            // The same for the window «Save» of «Export»: the tests of level 2 cannot drive a native window and manage theirs.
            app.manage(default_picker(app.handle()));
            // And the window «Open» of «Import».
            app.manage(default_file_picker(app.handle()));
            // What this launch is (T5): the files of its arguments are offered to the page, and the first window listens
            // for the files that later launches hand over. The tests of level 2 have no launch.
            if let Some(launch) = app.try_state::<Launch>() {
                let handle = app.handle().clone();
                events::offer_from_outside(&handle, &launch.files, import::Origin::Open);
                if let Some(primary) = launch.take_primary() {
                    tauri::async_runtime::spawn(primary.serve(move |files| {
                        let handle = handle.clone();
                        // reading the files is not quick: not on the thread of the channel
                        tauri::async_runtime::spawn_blocking(move || {
                            events::offer_from_outside(&handle, &files, import::Origin::Open);
                            window::raise(&handle);
                        });
                    }));
                }
            }
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

/// The window «Save» of the running app: the one of GTK on Linux, none elsewhere (an export is then a cancel).
#[cfg(target_os = "linux")]
fn default_picker<R: Runtime>(app: &tauri::AppHandle<R>) -> Arc<dyn SavePicker> {
    Arc::new(export_gtk::GtkSavePicker(app.clone()))
}

#[cfg(not(target_os = "linux"))]
fn default_picker<R: Runtime>(_app: &tauri::AppHandle<R>) -> Arc<dyn SavePicker> {
    Arc::new(export::NoPicker)
}

/// The window «Open» of the running app: the one of GTK on Linux, none elsewhere (a choice is then a cancel).
#[cfg(target_os = "linux")]
fn default_file_picker<R: Runtime>(app: &tauri::AppHandle<R>) -> Arc<dyn FilePicker> {
    Arc::new(picker_gtk::GtkFilePicker(app.clone()))
}

#[cfg(not(target_os = "linux"))]
fn default_file_picker<R: Runtime>(_app: &tauri::AppHandle<R>) -> Arc<dyn FilePicker> {
    Arc::new(picker::NoFilePicker)
}

/// What the launch of the process brought (plan B4, T5): the files of its arguments, and, in the first process, the channel
/// the next launches hand their files over. It is put in the state before the build; a process without it (the tests of
/// level 2) has no arguments and no channel.
pub struct Launch {
    pub files: Vec<PathBuf>,
    primary: std::sync::Mutex<Option<instance::Primary>>,
}

impl Launch {
    pub fn new(files: Vec<PathBuf>, primary: Option<instance::Primary>) -> Self {
        Self {
            files,
            primary: std::sync::Mutex::new(primary),
        }
    }

    fn take_primary(&self) -> Option<instance::Primary> {
        self.primary
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

/// Builds the Tauri application from `tauri.conf.json` and runs it until the last window closes. The daemon
/// socket is `GALLERIA_SOCKET` when set, the default path otherwise.
///
/// Before anything is built, the process finds out whether it is the first (`instance::claim`): a second launch gives its
/// files to the first window and ends here, so it does not even start WebKit. Without a runtime folder there is no channel:
/// the app starts alone and says so on `stderr`.
///
/// # Errors
///
/// Returns the Tauri error when the application cannot be built or crashes while running.
pub fn run() -> tauri::Result<std::process::ExitCode> {
    let socket = link::socket_path(std::env::var_os("GALLERIA_SOCKET"));
    let cwd = std::env::current_dir().unwrap_or_default();
    let files = instance::files_in_args(std::env::args_os().skip(1), &cwd);
    let dir = instance::runtime_dir(std::env::var_os("XDG_RUNTIME_DIR"));
    let primary = match instance::claim(dir.as_deref(), &files) {
        instance::Claim::Handed => return Ok(std::process::ExitCode::SUCCESS),
        instance::Claim::Failed(why) => {
            eprintln!("galleria-gui: {why}");
            return Ok(std::process::ExitCode::FAILURE);
        }
        instance::Claim::Alone(why) => {
            eprintln!("galleria-gui: no single window: {why}; the app starts on its own");
            None
        }
        instance::Claim::Primary(primary) => Some(primary),
    };
    let builder = tauri::Builder::default().manage(Launch::new(files, primary));
    configure(builder, socket).run(tauri::generate_context!())?;
    Ok(std::process::ExitCode::SUCCESS)
}
