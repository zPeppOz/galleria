//! The Windows body of the single instance: there is none yet (P6). Every launch is the first one, and works alone.
use super::Claim;
use std::{
    convert::Infallible,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// There is no first process on Windows, so a `Primary` cannot be made: the type only has to exist for `Claim`.
pub struct Primary {
    never: Infallible,
}

impl Primary {
    pub async fn serve(self, _on_files: impl Fn(Vec<PathBuf>) + Send + 'static) {
        match self.never {}
    }
}

/// Always `Alone`: no lock and no channel yet.
pub fn claim(_runtime_dir: Option<&Path>, _files: &[PathBuf]) -> Claim {
    Claim::Alone("the single instance is not there yet on Windows".to_owned())
}

/// The files given as arguments are not read on Windows yet.
pub fn files_in_args(_args: impl IntoIterator<Item = OsString>, _cwd: &Path) -> Vec<PathBuf> {
    Vec::new()
}
