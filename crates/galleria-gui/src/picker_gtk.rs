//! The window «Open» of GTK for «Import» (plan B4, T3, S27). Linux only, and with `export_gtk.rs` and `announce_atk.rs` the
//! only files of the crate that touch GTK. It is the window «Save» of «Export» with the action `Open` (see there for why
//! the plugin of Tauri is not used).

use crate::picker::FilePicker;
use gtk::prelude::{FileChooserExt, NativeDialogExt};
use std::{path::PathBuf, sync::mpsc};
use tauri::{AppHandle, Manager, Runtime};

/// The two filters of the window: the profiles, and every file. Their names are not translated (Rust has no catalogues,
/// D22): they are the patterns themselves.
const PROFILES: (&str, &[&str]) = ("*.ovpn, *.conf", &["*.ovpn", "*.conf"]);
const EVERY_FILE: (&str, &[&str]) = ("*", &["*"]);

/// Asks for the file on the main thread of GTK, over the window `main`, and waits for the answer on the thread of the
/// command (an `async` command runs off the main thread). The texts of the window (title and buttons) are GTK's own: they
/// come in the language of the system.
pub struct GtkFilePicker<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> FilePicker for GtkFilePicker<R> {
    fn pick(&self) -> Option<PathBuf> {
        let (answer, wait) = mpsc::channel();
        let app = self.0.clone();
        self.0
            .run_on_main_thread(move || {
                let parent = app
                    .get_webview_window("main")
                    .and_then(|window| window.gtk_window().ok());
                let chooser = gtk::FileChooserNative::new(
                    None,
                    parent.as_ref(),
                    gtk::FileChooserAction::Open,
                    None,
                    None,
                );
                for (name, patterns) in [PROFILES, EVERY_FILE] {
                    let filter = gtk::FileFilter::new();
                    filter.set_name(Some(name));
                    for pattern in patterns {
                        filter.add_pattern(pattern);
                    }
                    chooser.add_filter(filter.clone());
                    // the first one is the one shown
                    if name == PROFILES.0 {
                        chooser.set_filter(&filter);
                    }
                }
                let accepted = chooser.run() == gtk::ResponseType::Accept;
                let _ = answer.send(accepted.then(|| chooser.filename()).flatten());
            })
            .ok()?;
        // The event loop that stops before the window answers drops the sender: that is a cancel.
        wait.recv().ok().flatten()
    }
}
