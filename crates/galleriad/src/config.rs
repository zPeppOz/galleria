use std::{path::PathBuf, time::Duration};
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
            // Windows: the runtime folder is `%PROGRAMDATA%\Galleria`; making it with the right descriptor is P2-T3.
            #[cfg(windows)]
            runtime_dir: r"C:\ProgramData\Galleria".into(),
            #[cfg(unix)]
            runtime_dir: "/run/galleria".into(),
            hello_timeout: Duration::from_secs(5),
            max_clients: 16,
        }
    }
}
