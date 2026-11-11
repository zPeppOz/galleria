#![forbid(unsafe_code)]
#[cfg(feature = "fake")]
pub mod fake;
pub mod net;
use galleria_ipc::*;
use net::NetConfig;
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
pub const WG_INTERFACE: &str = "glr-wg0";
pub const OVPN_INTERFACE: &str = "glr-ovpn0";
pub const WG_FWMARK: u32 = 51820;
pub trait Backend: Send + 'static {
    fn protocol(&self) -> Protocol;
    fn capabilities(&self) -> Capabilities;
    fn spawn(self: Box<Self>, ctx: SessionContext) -> SessionHandle;
}
pub struct SessionContext {
    pub session: u64,
    pub session_dir: PathBuf,
    pub events: mpsc::Sender<BackendEvent>,
}
pub struct SessionHandle {
    pub commands: mpsc::Sender<BackendCommand>,
    pub task: JoinHandle<()>,
}
pub enum BackendCommand {
    Stop,
    ProvideInput(InputValues),
}
#[derive(Debug, Clone)]
pub struct TunnelInfo {
    pub local_addresses: Vec<IpAddr>,
    pub remote: Option<SocketAddr>,
}
#[derive(Debug, Clone)]
pub struct LogLine {
    pub level: LogLevel,
    pub message: String,
}
pub enum BackendEvent {
    Progress(String),
    InputRequired(InputRequest),
    NetUp {
        config: NetConfig,
        reply: oneshot::Sender<Result<(), String>>,
    },
    NetDown {
        reply: oneshot::Sender<()>,
    },
    Up(TunnelInfo),
    Restarting(String),
    Stats(Stats),
    Log(LogLine),
    Exited(ExitReason),
}
#[derive(Debug, Clone)]
pub enum ExitReason {
    Stopped,
    AuthFailed(String),
    Transport(String),
    Fatal(SessionError),
}
