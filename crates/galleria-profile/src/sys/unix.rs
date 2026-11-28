//! The part of the storage that touches the system, Unix body: modes `0600`/`0700`, the owner, `O_NOFOLLOW`.
use crate::StoreError;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
pub(crate) fn check(path: &Path, meta: &fs::Metadata) -> Result<(), StoreError> {
    if meta.file_type().is_symlink() {
        return Err(StoreError::Invalid(
            "Symbolic links are not allowed in profile storage".into(),
        ));
    }
    if meta.uid() != nix::unistd::geteuid().as_raw() {
        return Err(StoreError::WrongOwner { path: path.into() });
    }
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(StoreError::InsecurePermissions {
            path: path.into(),
            mode,
        });
    }
    Ok(())
}
pub(crate) fn directory(path: &Path) -> Result<(), StoreError> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
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
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
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
    let dir = path
        .parent()
        .ok_or_else(|| StoreError::Invalid("Missing storage directory".into()))?;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| StoreError::Io(e.error))?;
    File::open(dir)?.sync_all()?;
    Ok(())
}
/// `$XDG_CONFIG_HOME/galleria`, or `$HOME/.config/galleria`.
pub(crate) fn default_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir).join("galleria");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/galleria")
}
/// Opens the file that is to be imported: without blocking (a FIFO that nobody writes to would wait for ever otherwise).
pub(crate) fn open_for_import(path: &Path) -> std::io::Result<File> {
    let flags = i32::try_from(rustix::fs::OFlags::NONBLOCK.bits()).unwrap_or(0);
    OpenOptions::new().read(true).custom_flags(flags).open(path)
}
