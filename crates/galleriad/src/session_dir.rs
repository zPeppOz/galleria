use std::{
    fs::{self, DirBuilder},
    io,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
};
fn private_dir(path: &Path) -> io::Result<()> {
    DirBuilder::new().mode(0o700).create(path)
}
fn session_root(runtime_dir: &Path) -> io::Result<PathBuf> {
    let root = runtime_dir.join("s");
    match fs::symlink_metadata(&root) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }
        Ok(_) => return Err(io::Error::other("Invalid session root")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => private_dir(&root)?,
        Err(e) => return Err(e),
    }
    Ok(root)
}
pub fn create_session_dir(runtime_dir: &Path) -> io::Result<PathBuf> {
    create_named_session_dir(runtime_dir, uuid::Uuid::new_v4())
}
fn create_named_session_dir(runtime_dir: &Path, id: uuid::Uuid) -> io::Result<PathBuf> {
    let root = session_root(runtime_dir)?;
    let path = root.join(id.to_string());
    private_dir(&path)?;
    Ok(path)
}
pub fn purge_session_dirs(runtime_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(runtime_dir)?;
    let root = runtime_dir.join("s");
    match fs::symlink_metadata(&root) {
        Ok(meta) => {
            if meta.is_dir() && !meta.file_type().is_symlink() {
                fs::remove_dir_all(&root)?;
            } else {
                fs::remove_file(&root)?;
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    private_dir(&root)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn existing_name_is_never_reused() {
        let tmp = tempfile::tempdir().unwrap();
        let id = uuid::Uuid::new_v4();
        let p = create_named_session_dir(tmp.path(), id).unwrap();
        fs::write(p.join("keep"), b"keep").unwrap();
        assert_eq!(
            create_named_session_dir(tmp.path(), id).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(p.join("keep")).unwrap(), b"keep");
    }
}
