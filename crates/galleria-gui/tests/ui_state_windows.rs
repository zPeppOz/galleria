#![cfg(windows)]
//! `state.toml` on Windows (T5): where it lives, and that what is written is read back. The rest of the proofs of the
//! file are in `ui_state.rs`, which is a Linux file for now.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleria_gui::ui_state::{SortOrder, ThemeChoice, UiStateFile, state_path};
use std::{ffi::OsString, path::Path};

#[test]
fn the_state_file_lives_in_localappdata() {
    let path = state_path(Some(OsString::from(r"C:\Users\x\AppData\Local"))).unwrap();
    assert_eq!(
        path,
        Path::new(r"C:\Users\x\AppData\Local\galleria\state.toml")
    );
    for bad in [None, Some(""), Some("state"), Some(r".\x"), Some("C:x")] {
        assert_eq!(state_path(bad.map(OsString::from)), None, "{bad:?}");
    }
}

#[test]
fn the_state_is_written_and_read_back_on_windows() {
    let dir = tempfile::tempdir().unwrap();
    let file = UiStateFile::new(dir.path().join(r"state\galleria\state.toml"));
    file.update(|s| {
        s.theme = ThemeChoice::Dark;
        s.sort = SortOrder::Recent;
    })
    .unwrap();
    let back = file.load();
    assert_eq!(back.theme, ThemeChoice::Dark);
    assert_eq!(back.sort, SortOrder::Recent);
    // a second write replaces the file: one file in the folder, the new content
    file.update(|s| s.theme = ThemeChoice::Light).unwrap();
    assert_eq!(file.load().theme, ThemeChoice::Light);
    let names: Vec<_> = std::fs::read_dir(dir.path().join(r"state\galleria"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["state.toml"]);
}

#[test]
fn an_unusable_variable_is_named_by_the_error_of_the_write() {
    let file = UiStateFile::new(std::path::PathBuf::new());
    assert_eq!(file.load(), galleria_gui::ui_state::UiState::default());
    let error = file.update(|s| s.theme = ThemeChoice::Dark).unwrap_err();
    assert!(error.to_string().contains("%LOCALAPPDATA%"), "{error}");
}
