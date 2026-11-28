#![cfg(windows)]
//! T5: the folder of the service on Windows is `%PROGRAMDATA%\Galleria`. Only the path; making it with the right
//! descriptor is P2-T3.
use galleriad::config::service_dir;
use std::{ffi::OsString, path::Path};

#[test]
fn the_service_folder_is_under_programdata() {
    assert_eq!(
        service_dir(Some(OsString::from(r"C:\ProgramData"))).as_deref(),
        Some(Path::new(r"C:\ProgramData\Galleria"))
    );
    for bad in [
        None,
        Some(""),
        Some("ProgramData"),
        Some(r".\x"),
        Some("C:x"),
    ] {
        assert_eq!(service_dir(bad.map(OsString::from)), None, "{bad:?}");
    }
}
