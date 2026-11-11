#![forbid(unsafe_code)]
pub mod journal;
pub mod manager;
pub mod plan;
pub mod runner;
pub use manager::NetManager;
pub use plan::{Cmd, Program, Step, fixed_cleanup, plan};
pub use runner::{CommandRunner, SystemRunner};

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("DNS service unavailable: {0}")]
    DnsUnavailable(String),
    #[error("Network command failed: {cmd}: {stderr}")]
    Command { cmd: String, stderr: String },
    #[error("Network journal failed")]
    Journal(#[from] std::io::Error),
    #[error("Required tool missing: {0}")]
    ToolMissing(&'static str),
    #[error("Invalid network configuration: {0}")]
    InvalidConfig(&'static str),
}
