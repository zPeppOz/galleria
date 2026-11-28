use std::{path::PathBuf, time::Duration};

/// Windows: `%PROGRAMDATA%\Galleria`, the folder of the service. Only the path: making it with the right descriptor is
/// P2-T3. Pure, like the paths of the profiles: a value that is missing, empty or not absolute gives `None`.
#[cfg(windows)]
#[must_use]
pub fn service_dir(programdata: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let base = PathBuf::from(programdata.filter(|v| !v.is_empty())?);
    base.is_absolute().then(|| base.join("Galleria"))
}

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub socket: PathBuf,
    pub group: String,
    pub runtime_dir: PathBuf,
    pub hello_timeout: Duration,
    pub max_clients: usize,
}
impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket: galleria_ipc::DEFAULT_SOCKET_PATH.into(),
            group: "galleria".into(),
            // Windows: `%PROGRAMDATA%\Galleria`; where the variable cannot be used, the folder where it normally is.
            #[cfg(windows)]
            runtime_dir: service_dir(std::env::var_os("PROGRAMDATA"))
                .unwrap_or_else(|| r"C:\ProgramData\Galleria".into()),
            #[cfg(unix)]
            runtime_dir: "/run/galleria".into(),
            hello_timeout: Duration::from_secs(5),
            max_clients: 16,
        }
    }
}
