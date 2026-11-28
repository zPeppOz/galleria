#![forbid(unsafe_code)]
pub mod device;
#[cfg(target_os = "linux")]
mod kernel;
pub mod session;
pub mod validate;
pub use device::{DeviceConfig, DevicePeer, DeviceStats, WgDevice, WgError};
#[cfg(target_os = "linux")]
pub use kernel::KernelDevice;
pub use session::WireGuardBackend;
pub use validate::{ValidWireGuard, net_config, validate};
