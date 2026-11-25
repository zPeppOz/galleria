//! The window «Save» of GTK for «Export» (plan B3, T8, S22). Linux only, and the only other file of the crate that touches
//! GTK besides `announce_atk.rs`.
//!
//! Why not the plugin of Tauri (S22): `tauri-plugin-dialog` 2.8.4 is `Apache-2.0 OR MIT` and works from Rust with no
//! permission for the webview, but it brings `rfd`, `tauri-plugin-fs` and `tauri-plugin` with it (four crates and a second
//! major version of `toml` in `Cargo.lock`), registers the commands of two plugins in the app (denied by the capability, but
//! there), and its `gtk3` backend is `GtkFileChooserNative` itself. `gtk` is already a dependency (Q27): the same dialog
//! costs no crate and no command.

use crate::export::SavePicker;
use gtk::prelude::{FileChooserExt, NativeDialogExt};
use std::{path::PathBuf, sync::mpsc};
use tauri::{AppHandle, Manager, Runtime};

/// Asks for the place on the main thread of GTK, over the window `main`, and waits for the answer on the thread of the
/// command (an `async` command runs off the main thread). The texts of the window (title and buttons) are GTK's own: they
/// come in the language of the system, and nothing of Galleria's catalogues is needed for them.
pub struct GtkSavePicker<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> SavePicker for GtkSavePicker<R> {
    fn pick(&self, suggested_name: &str) -> Option<PathBuf> {
        let (answer, wait) = mpsc::channel();
        let app = self.0.clone();
        let name = suggested_name.to_owned();
        self.0
            .run_on_main_thread(move || {
                let parent = app
                    .get_webview_window("main")
                    .and_then(|window| window.gtk_window().ok());
                let chooser = gtk::FileChooserNative::new(
                    None,
                    parent.as_ref(),
                    gtk::FileChooserAction::Save,
                    None,
                    None,
                );
                chooser.set_do_overwrite_confirmation(true);
                chooser.set_current_name(&name);
                let accepted = chooser.run() == gtk::ResponseType::Accept;
                let _ = answer.send(accepted.then(|| chooser.filename()).flatten());
            })
            .ok()?;
        // The event loop that stops before the window answers drops the sender: that is a cancel.
        wait.recv().ok().flatten()
    }
}
