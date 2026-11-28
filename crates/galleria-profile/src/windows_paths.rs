//! Windows: the folder of the profiles. Pure, like `state_path` of the window: the value of the variable comes in as an
//! argument, so that it can be tried without touching the environment of the test process.
//!
//! A value that is missing, empty or not absolute (`profili`, `.\x`, `C:x`, `\x`) gives `None`: a relative folder would
//! depend on where the program was started from. Whoever calls answers with an error that names the variable.
use std::{ffi::OsString, path::PathBuf};

/// The value of a folder variable when it is usable: present, not empty, absolute.
#[must_use]
pub fn absolute_folder(value: Option<OsString>) -> Option<PathBuf> {
    let folder = PathBuf::from(value.filter(|v| !v.is_empty())?);
    folder.is_absolute().then_some(folder)
}

/// `%APPDATA%\galleria`: the profiles. `%APPDATA%` is the folder of the user alone (spec §8), so no permission is written.
#[must_use]
pub fn profiles_dir(appdata: Option<OsString>) -> Option<PathBuf> {
    absolute_folder(appdata).map(|dir| dir.join("galleria"))
}

/// The text of the error of a variable that cannot be used.
#[must_use]
pub fn unusable(variable: &str) -> String {
    format!(
        "%{variable}% is missing, empty or not an absolute path: the folder of galleria cannot be told"
    )
}
