//! The part of the storage that touches the system, Windows body. For now the minimum that compiles: every function
//! says "not supported". The paths and the protection of the files are written in T5, with their proofs.
use crate::StoreError;
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "galleria: the profile storage is not there yet on Windows",
    )
}
pub(crate) fn check(_path: &Path, _meta: &fs::Metadata) -> Result<(), StoreError> {
    Err(StoreError::Io(unsupported()))
}
pub(crate) fn directory(_path: &Path) -> Result<(), StoreError> {
    Err(StoreError::Io(unsupported()))
}
pub(crate) fn read_private(_path: &Path) -> Result<String, StoreError> {
    Err(StoreError::Io(unsupported()))
}
pub(crate) fn atomic(_path: &Path, _text: &str) -> Result<(), StoreError> {
    Err(StoreError::Io(unsupported()))
}
pub(crate) fn default_dir() -> PathBuf {
    PathBuf::new()
}
pub(crate) fn open_for_import(_path: &Path) -> io::Result<File> {
    Err(unsupported())
}
