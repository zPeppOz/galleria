//! Import from a file or from pasted text (plan B4, T1): what the window needs on top of the two import functions of the
//! phase 1, which stay as they are (Q29). Nothing in here writes: the result is a profile and its secrets in memory.
//!
//! A path that comes from outside is not trusted (a FIFO, a device, a huge file, a link): the file is opened without
//! blocking, the **open file** is looked at (never the path, so that what is checked is what is read), and it must be a
//! regular file of at most [`IMPORT_MAX_BYTES`]. The files that a `.ovpn` refers to are read only from the real folder
//! of the profile, with the same ceiling, as the CLI does (spec F1 §8.2).

use crate::{ImportError, Imported, import_openvpn_bytes, import_wireguard};
use galleria_ipc::Protocol;
use std::{
    cell::Cell,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

/// The most a profile, or a file it refers to, may weigh: 1 MiB. A `.ovpn` with four PEM blocks is a few KiB.
pub const IMPORT_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ImportFileError {
    #[error("Cannot read the file")]
    Unreadable(#[source] std::io::Error),
    #[error("Not a regular file")]
    NotAFile,
    #[error("The file is too large")]
    TooLarge,
    #[error("Not a .ovpn or .conf profile")]
    UnsupportedFile,
    #[error(transparent)]
    Import(#[from] ImportError),
}

/// WireGuard if the first line that is neither empty nor a comment (`#` or `;`) is `[Interface]`, whatever the case
/// (`wg-quick` reads it so); OpenVPN otherwise. Only the first meaningful line counts: a `.ovpn` that has
/// «[Interface]» in a comment, or later, is an OpenVPN profile.
pub fn detect_protocol(text: &str) -> Protocol {
    let first = text
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with(';'));
    if first.is_some_and(|line| line.eq_ignore_ascii_case("[Interface]")) {
        Protocol::Wireguard
    } else {
        Protocol::Openvpn
    }
}

/// The profile in a text, with no folder: a reference to a file gives [`ImportError::MissingFile`].
///
/// # Errors
///
/// The errors of the import of the phase 1.
pub fn import_text(text: &str, name: &str) -> Result<Imported, ImportError> {
    match detect_protocol(text) {
        Protocol::Wireguard => import_wireguard(text, name),
        Protocol::Openvpn => import_openvpn_bytes(text, name, &|_| {
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        }),
    }
}

/// Opens `path` without blocking and reads at most the ceiling of a regular file.
fn read_regular(path: &Path) -> Result<Zeroizing<Vec<u8>>, ImportFileError> {
    let file = crate::sys::open_for_import(path).map_err(ImportFileError::Unreadable)?;
    let meta = file.metadata().map_err(ImportFileError::Unreadable)?;
    if !meta.is_file() {
        return Err(ImportFileError::NotAFile);
    }
    if meta.len() > IMPORT_MAX_BYTES {
        return Err(ImportFileError::TooLarge);
    }
    read_capped(file)
}

fn read_capped(file: File) -> Result<Zeroizing<Vec<u8>>, ImportFileError> {
    let mut bytes = Zeroizing::new(Vec::new());
    // One byte more than the ceiling, to tell a file that grew from one that fits.
    file.take(IMPORT_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(ImportFileError::Unreadable)?;
    if bytes.len() as u64 > IMPORT_MAX_BYTES {
        return Err(ImportFileError::TooLarge);
    }
    Ok(bytes)
}

/// Why a file that the profile refers to was not read, kept apart from the generic error the import gives.
#[derive(Clone, Copy)]
enum Refused {
    TooLarge,
    Outside,
}

/// The default name of the profile: the name of the file without its extension.
fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Reads and imports a `.ovpn` or a `.conf`. `.ovpn` is OpenVPN; a `.conf` is told apart by its content (Q39); any other
/// extension is [`ImportFileError::UnsupportedFile`]. The files a profile refers to are read only from the real folder
/// of the profile (the links are resolved first) and have the same ceiling.
///
/// # Errors
///
/// [`ImportFileError`]: the file cannot be read, is not a regular file, is too large, has another extension, or the
/// profile does not import.
pub fn import_file(path: &Path, name: Option<&str>) -> Result<Imported, ImportFileError> {
    let text = read_import_file(path)?;
    import_read_file(path, &text, name)
}

/// The text of a file to import: the extension is looked at first, then the file is read as [`import_file`] reads it.
/// The window reads the text itself to be able to look at it again (which script a removed directive was) before it is
/// let go.
///
/// # Errors
///
/// [`ImportFileError::UnsupportedFile`], [`ImportFileError::NotAFile`], [`ImportFileError::TooLarge`] and
/// [`ImportFileError::Unreadable`] (also for a text that is not UTF-8).
pub fn read_import_file(path: &Path) -> Result<Zeroizing<String>, ImportFileError> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("ovpn" | "conf")) {
        return Err(ImportFileError::UnsupportedFile);
    }
    let bytes = read_regular(path)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        ImportFileError::Unreadable(std::io::Error::from(std::io::ErrorKind::InvalidData))
    })?;
    Ok(Zeroizing::new(text.to_owned()))
}

/// The import of the text that [`read_import_file`] read from `path`: the folder of `path` is where the files the profile
/// refers to are looked for.
///
/// # Errors
///
/// The errors of [`import_file`] after the reading.
pub fn import_read_file(
    path: &Path,
    text: &str,
    name: Option<&str>,
) -> Result<Imported, ImportFileError> {
    let by_content = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("conf"));
    let name = name.map_or_else(|| stem(path), str::to_owned);
    if by_content && detect_protocol(text) == Protocol::Wireguard {
        return Ok(import_wireguard(text, &name)?);
    }
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let root: PathBuf = std::fs::canonicalize(folder).map_err(ImportFileError::Unreadable)?;
    let refused: Cell<Option<Refused>> = Cell::new(None);
    let imported = import_openvpn_bytes(text, &name, &|reference| {
        let real = std::fs::canonicalize(root.join(reference))?;
        if !real.starts_with(&root) {
            refused.set(Some(Refused::Outside));
            return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        }
        match read_regular(&real) {
            Ok(mut bytes) => Ok(std::mem::take(&mut *bytes)),
            Err(ImportFileError::TooLarge) => {
                refused.set(Some(Refused::TooLarge));
                Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
            }
            Err(ImportFileError::Unreadable(error)) => Err(error),
            Err(_) => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
        }
    });
    match (imported, refused.get()) {
        (Ok(imported), _) => Ok(imported),
        (Err(ImportError::MissingFile(_)), Some(Refused::TooLarge)) => {
            Err(ImportFileError::TooLarge)
        }
        (Err(ImportError::MissingFile(file)), Some(Refused::Outside)) => {
            Err(ImportError::OutsideFolder(file).into())
        }
        (Err(error), _) => Err(error.into()),
    }
}
