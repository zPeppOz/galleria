#![deny(unsafe_code)]
// libc::getsockopt is needed: rustix 1.1.6 and nix 0.31.3 have no safe SO_PEERGROUPS API.
pub mod auth;
pub mod config;
mod daemon_info;
pub mod dispatch;
pub mod factory;
pub mod logbuf;
pub mod machine;
pub mod manager;
pub mod network;
#[cfg(unix)]
#[allow(unsafe_code)]
mod peer_groups;
pub mod redact;
#[cfg(unix)]
pub mod server;
/// Windows: there is no server yet (the service is P2), only the description of the daemon that `dispatch` asks for.
#[cfg(windows)]
pub mod server {
    pub use crate::daemon_info::DaemonInfo;
}
pub mod session_dir;

/// Failure is visible but does not prevent startup, as specified for process hardening.
#[cfg(unix)]
pub fn harden_process() {
    if rustix::process::set_dumpable_behavior(rustix::process::DumpableBehavior::NotDumpable)
        .is_err()
    {
        eprintln!("warning: could not disable process core dumps");
    }
}
