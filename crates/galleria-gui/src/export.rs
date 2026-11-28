//! «Export» in the window (plan B3, T8, Q31): where the file goes and how it is written. No type of Tauri in here: the
//! window that asks for the place is a `SavePicker`, so that the tests (which cannot drive a native window) give their
//! own, and the file never passes through the webview.

use galleria_ipc::Protocol;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// The window «Save» of the system: asks where to put the file, offering a name. `None` is a cancel, and a cancel is not
/// an error. The real one is `export_gtk::GtkSavePicker`; the tests replace it.
pub trait SavePicker: Send + Sync {
    fn pick(&self, suggested_name: &str) -> Option<PathBuf>;
}

/// The picker of a system with no window to ask: it always cancels.
pub struct NoPicker;

impl SavePicker for NoPicker {
    fn pick(&self, _suggested_name: &str) -> Option<PathBuf> {
        None
    }
}

/// Why the file could not be written. It names the place, never the text.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Cannot write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl ExportError {
    pub fn path(&self) -> &Path {
        match self {
            Self::Io { path, .. } => path,
        }
    }
}

/// The longest name offered, in characters, before the extension. A file name is at most 255 bytes on the usual file
/// systems and a character is up to four.
const NAME_CHARS: usize = 100;

/// The name offered in the window «Save»: the name of the profile with `.ovpn` or `.conf`. A slash, a backslash and a
/// control character become `_`; dots at the start go (no hidden file, no `..`); what is left empty is `profile`; and a
/// long name is cut.
pub fn suggested_name(profile_name: &str, protocol: Protocol) -> String {
    let cleaned: String = profile_name
        .trim()
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim_start_matches('.').trim();
    let stem: String = cleaned.chars().take(NAME_CHARS).collect();
    let stem = if stem.is_empty() { "profile" } else { &stem };
    let extension = match protocol {
        Protocol::Openvpn => "ovpn",
        Protocol::Wireguard => "conf",
    };
    format!("{stem}.{extension}")
}

/// Writes the exported text at `path`: a private temporary file next to it, `0600`, written and synced, then renamed over
/// the place, so that nobody ever reads half of it and a file that was there is replaced whole. The text holds keys: the
/// file is private from the first byte (a temporary file is created `0600`).
///
/// # Errors
///
/// [`ExportError::Io`] with the place, when the folder is not there or not writable or the disk fails.
pub fn write_export(path: &Path, text: &str) -> Result<(), ExportError> {
    let fail = |source| ExportError::Io {
        path: path.to_owned(),
        source,
    };
    let folder = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut file = tempfile::NamedTempFile::new_in(folder).map_err(fail)?;
    // Windows: the file is written without `chmod`; a file made in a folder of the user is the user's.
    #[cfg(unix)]
    file.as_file()
        .set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(fail)?;
    file.write_all(text.as_bytes()).map_err(fail)?;
    file.as_file().sync_all().map_err(fail)?;
    file.persist(path).map_err(|e| fail(e.error))?;
    Ok(())
}
