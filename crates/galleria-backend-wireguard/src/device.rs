//! The only adapter importing the external WireGuard implementation.
use base64::{Engine, engine::general_purpose::STANDARD};
use defguard_wireguard_rs::{
    InterfaceConfiguration, Kernel, WGApi, WireguardInterfaceApi, key::Key, net::IpAddrMask,
    peer::Peer,
};
use galleria_backend::WG_INTERFACE;
use ipnet::IpNet;
use std::{net::SocketAddr, time::SystemTime};
use zeroize::{Zeroize, Zeroizing};
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
pub struct KernelDevice {
    api: WGApi<Kernel>,
    first_peer: Option<[u8; 32]>,
}
impl KernelDevice {
    pub fn new() -> Result<Self, WgError> {
        Ok(Self {
            api: WGApi::new(WG_INTERFACE).map_err(|_| WgError("initialize"))?,
            first_peer: None,
        })
    }
}
fn mask(n: &IpNet) -> IpAddrMask {
    IpAddrMask::new(n.addr(), n.prefix_len())
}
impl WgDevice for KernelDevice {
    fn create(&mut self) -> Result<(), WgError> {
        self.api.create_interface().map_err(|_| WgError("create"))
    }
    fn configure(&mut self, cfg: &DeviceConfig) -> Result<(), WgError> {
        let peers = cfg
            .peers
            .iter()
            .map(|p| {
                let mut peer = Peer::new(Key::new(p.public_key));
                peer.preshared_key = p.preshared_key.as_ref().map(|k| Key::new(**k));
                peer.endpoint = p.endpoint;
                peer.allowed_ips = p.allowed_ips.iter().map(mask).collect();
                peer.persistent_keepalive_interval = p.persistent_keepalive;
                peer
            })
            .collect();
        self.first_peer = cfg.peers.first().map(|p| p.public_key);
        let mut config = InterfaceConfiguration {
            name: WG_INTERFACE.into(),
            prvkey: STANDARD.encode(cfg.private_key.as_slice()),
            addresses: cfg.addresses.iter().map(mask).collect(),
            port: 0,
            peers,
            mtu: cfg.mtu,
            fwmark: cfg.fwmark,
        };
        let result = self
            .api
            .configure_interface(&config)
            .map_err(|_| WgError("configure"));
        config.prvkey.zeroize();
        result
    }
    fn read(&self) -> Result<DeviceStats, WgError> {
        let host = self
            .api
            .read_interface_data()
            .map_err(|_| WgError("read"))?;
        Ok(DeviceStats {
            rx_bytes: host.peers.values().map(|p| p.rx_bytes).sum(),
            tx_bytes: host.peers.values().map(|p| p.tx_bytes).sum(),
            last_handshake: self
                .first_peer
                .and_then(|key| host.peers.get(&Key::new(key)))
                .and_then(|p| p.last_handshake)
                .filter(|t| *t > SystemTime::UNIX_EPOCH),
        })
    }
    fn remove(&mut self) -> Result<(), WgError> {
        self.api.remove_interface().map_err(|_| WgError("remove"))
    }
}
