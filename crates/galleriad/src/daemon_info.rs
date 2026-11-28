//! What the daemon says about itself, kept apart from the server so that the Windows build has it too.
use galleria_ipc::BackendInfo;
#[derive(Clone)]
pub struct DaemonInfo {
    pub version: String,
    pub backends: Vec<BackendInfo>,
    pub backend_probe: Option<std::sync::Arc<dyn Fn() -> Vec<BackendInfo> + Send + Sync>>,
}
