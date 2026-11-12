#![forbid(unsafe_code)]
pub mod device;
pub mod session;
pub mod validate;
pub use device::{DeviceConfig, DevicePeer, DeviceStats, KernelDevice, WgDevice, WgError};
pub use session::WireGuardBackend;
pub use validate::{ValidWireGuard, net_config, validate};
