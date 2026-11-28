//! The part of the storage that touches the system, Windows body. No permission is written by us: `%APPDATA%` is of
//! the user alone (spec §8), and what is made inside inherits that. The temporary file is born next to the final one.
//! A link (symbolic link or junction) is refused, as on Unix, and a file is opened as itself, not through the link.
use crate::{StoreError, windows_paths};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

/// `FILE_FLAG_OPEN_REPARSE_POINT`: a link is opened as itself, not followed.
const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
/// `FILE_FLAG_BACKUP_SEMANTICS`: without it a folder cannot be opened at all («access denied»), and a dropped folder would be
/// «unreadable» instead of «not a file», which is what Linux says.
const BACKUP_SEMANTICS: u32 = 0x0200_0000;

pub(crate) fn check(_path: &Path, meta: &fs::Metadata) -> Result<(), StoreError> {
    if meta.file_type().is_symlink() {
        return Err(StoreError::Invalid(
            "Symbolic links are not allowed in profile storage".into(),
        ));
    }
    Ok(())
}
pub(crate) fn directory(path: &Path) -> Result<(), StoreError> {
    if path.as_os_str().is_empty() {
        return Err(StoreError::Invalid(windows_paths::unusable("APPDATA")));
    }
    fs::create_dir_all(path)?;
    let meta = fs::symlink_metadata(path)?;
    check(path, &meta)?;
    if !meta.is_dir() {
        return Err(StoreError::Invalid("Expected a directory".into()));
    }
    Ok(())
}
pub(crate) fn read_private(path: &Path) -> Result<String, StoreError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(OPEN_REPARSE_POINT)
        .open(path)?;
    let meta = file.metadata()?;
    check(path, &meta)?;
    if !meta.is_file() {
        return Err(StoreError::Invalid("Expected a regular file".into()));
    }
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}
pub(crate) fn atomic(path: &Path, text: &str) -> Result<(), StoreError> {
    atomic_with(path, text, |_| {})
}
/// [`atomic`], showing the temporary file to `seen` before it takes the place of the final one (the proof that it is
/// next to it).
fn atomic_with(path: &Path, text: &str, seen: impl FnOnce(&Path)) -> Result<(), StoreError> {
    let dir = path
        .parent()
        .ok_or_else(|| StoreError::Invalid("Missing storage directory".into()))?;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    seen(file.path());
    file.persist(path).map_err(|e| StoreError::Io(e.error))?;
    Ok(())
}
/// `%APPDATA%\galleria`; empty when the variable cannot be used, and then [`directory`] says so.
pub(crate) fn default_dir() -> PathBuf {
    windows_paths::profiles_dir(std::env::var_os("APPDATA")).unwrap_or_default()
}
/// Opens the file that is to be imported as itself: a link is not followed, and `read_regular` refuses it, and a folder,
/// for not being a regular file.
pub(crate) fn open_for_import(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(OPEN_REPARSE_POINT | BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn the_temporary_file_is_made_next_to_the_final_one() {
        let dir = tempfile::tempdir().unwrap();
        let profiles = dir.path().join("profiles");
        directory(&profiles).unwrap();
        let target = profiles.join("one.toml");
        let mut seen_in = None;
        atomic_with(&target, "a = 1\n", |temp| {
            assert!(temp.exists(), "the temporary file exists before the rename");
            assert_ne!(temp, target);
            seen_in = temp.parent().map(Path::to_path_buf);
        })
        .unwrap();
        assert_eq!(seen_in.as_deref(), Some(profiles.as_path()));
        let names: Vec<_> = fs::read_dir(&profiles)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["one.toml"], "nothing is left beside the final file");
    }
}
