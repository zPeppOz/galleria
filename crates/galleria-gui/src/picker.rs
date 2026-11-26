//! The window «Open» of the system for «Import» (plan B4, T3, S27): where the file to import is chosen. No type of Tauri in
//! here: the window is a `FilePicker`, so that the tests (which cannot drive a native window) give their own, and the path
//! of the file never passes through the webview (spec §3.2, rule 3).

use std::path::PathBuf;

/// The window «Open» of the system: asks for a file. `None` is a cancel, and a cancel is not an error. The real one is
/// `picker_gtk::GtkFilePicker`; the tests replace it.
pub trait FilePicker: Send + Sync {
    fn pick(&self) -> Option<PathBuf>;
}

/// The picker of a system with no window to ask: it always cancels.
pub struct NoFilePicker;

impl FilePicker for NoFilePicker {
    fn pick(&self) -> Option<PathBuf> {
        None
    }
}
