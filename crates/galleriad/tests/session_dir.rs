#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleriad::session_dir::*;
use std::{
    collections::HashSet,
    os::unix::fs::{PermissionsExt, symlink},
};
#[test]
fn session_dir_names_are_unique_and_0700() {
    let tmp = tempfile::tempdir().unwrap();
    let mut seen = HashSet::new();
    for _ in 0..1000 {
        let p = create_session_dir(tmp.path()).unwrap();
        assert!(seen.insert(p.clone()));
        assert_eq!(
            std::fs::metadata(p).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
#[test]
fn existing_name_is_never_reused() {
    let tmp = tempfile::tempdir().unwrap();
    let p = create_session_dir(tmp.path()).unwrap();
    std::fs::write(p.join("keep"), b"keep").unwrap();
    let q = create_session_dir(tmp.path()).unwrap();
    assert_ne!(p, q);
    assert!(p.join("keep").exists());
}
#[test]
fn purge_removes_orphans_and_does_not_follow_symlinks() {
    let tmp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::fs::write(external.path().join("keep"), b"keep").unwrap();
    let p = create_session_dir(tmp.path()).unwrap();
    std::fs::write(p.join("mgmt.sock"), b"orphan").unwrap();
    symlink(external.path(), tmp.path().join("s/link")).unwrap();
    purge_session_dirs(tmp.path()).unwrap();
    assert_eq!(std::fs::read_dir(tmp.path().join("s")).unwrap().count(), 0);
    assert!(external.path().join("keep").exists());
    std::fs::remove_dir(tmp.path().join("s")).unwrap();
    symlink(external.path(), tmp.path().join("s")).unwrap();
    purge_session_dirs(tmp.path()).unwrap();
    assert!(external.path().join("keep").exists());
    assert!(
        !std::fs::symlink_metadata(tmp.path().join("s"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
