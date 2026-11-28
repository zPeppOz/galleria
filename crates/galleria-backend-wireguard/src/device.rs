//! The device interface of the WireGuard backend; the implementation of the Linux kernel is in `kernel`.
use ipnet::IpNet;
use std::{net::SocketAddr, time::SystemTime};
use zeroize::Zeroizing;
#[derive(Debug, thiserror::Error)]
#[error("WireGuard device operation failed: {0}")]
pub struct WgError(pub &'static str);
pub struct DevicePeer {
    pub public_key: [u8; 32],
    pub preshared_key: Option<Zeroizing<[u8; 32]>>,
    pub endpoint: Option<SocketAddr>,
    pub allowed_ips: Vec<IpNet>,
    pub persistent_keepalive: Option<u16>,
}
pub struct DeviceConfig {
    pub private_key: Zeroizing<[u8; 32]>,
    pub addresses: Vec<IpNet>,
    pub mtu: Option<u32>,
    pub peers: Vec<DevicePeer>,
    pub fwmark: Option<u32>,
}
pub struct DeviceStats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub last_handshake: Option<SystemTime>,
}
pub trait WgDevice: Send + 'static {
    fn create(&mut self) -> Result<(), WgError>;
    fn configure(&mut self, cfg: &DeviceConfig) -> Result<(), WgError>;
    fn read(&self) -> Result<DeviceStats, WgError>;
    fn remove(&mut self) -> Result<(), WgError>;
}
