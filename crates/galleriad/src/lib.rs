#![deny(unsafe_code)]
// libc::getsockopt is needed: rustix 1.1.6 and nix 0.31.3 have no safe SO_PEERGROUPS API.
pub mod auth;
pub mod config;
pub mod dispatch;
pub mod machine;
pub mod manager;
#[allow(unsafe_code)]
mod peer_groups;
pub mod server;
pub mod session_dir;

/// Failure is visible but does not prevent startup, as specified for process hardening.
pub fn harden_process() {
    if rustix::process::set_dumpable_behavior(rustix::process::DumpableBehavior::NotDumpable)
        .is_err()
    {
        eprintln!("warning: could not disable process core dumps");
    }
}
