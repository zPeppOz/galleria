//! `KernelDevice`: the WireGuard of the Linux kernel, through Defguard and netlink. Linux only.
use crate::device::{DeviceConfig, DeviceStats, WgDevice, WgError};
use base64::{Engine, engine::general_purpose::STANDARD};
use defguard_wireguard_rs::{
    InterfaceConfiguration, Kernel, WGApi, WireguardInterfaceApi, key::Key, net::IpAddrMask,
    peer::Peer,
};
use galleria_backend::WG_INTERFACE;
use ipnet::IpNet;
use std::time::SystemTime;
use zeroize::Zeroize;
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
        // Defguard's removal invokes unbounded DNS helpers and also owns policy rules.
        // The daemon already tears those down; delete only our link with bounded netlink.
        delete_link()
    }
}
fn delete_link() -> Result<(), WgError> {
    use netlink_packet_core::{NLM_F_ACK, NLM_F_REQUEST, NetlinkMessage, NetlinkPayload};
    use netlink_packet_route::{
        RouteNetlinkMessage,
        link::{LinkAttribute, LinkMessage},
    };
    use netlink_sys::{Socket, SocketAddr, protocols::NETLINK_ROUTE};
    use std::{
        io::ErrorKind,
        time::{Duration, Instant},
    };
    let failure = || WgError("remove");
    let mut link = LinkMessage::default();
    link.attributes
        .push(LinkAttribute::IfName(WG_INTERFACE.into()));
    let mut request = NetlinkMessage::from(RouteNetlinkMessage::DelLink(link));
    request.header.flags = NLM_F_REQUEST | NLM_F_ACK;
    request.header.sequence_number = 1;
    request.finalize();
    let mut buffer = vec![0; request.buffer_len()];
    request.serialize(&mut buffer);
    let socket = Socket::new(NETLINK_ROUTE).map_err(|_| failure())?;
    socket
        .connect(&SocketAddr::new(0, 0))
        .map_err(|_| failure())?;
    socket.set_non_blocking(true).map_err(|_| failure())?;
    if socket.send(&buffer, 0).map_err(|_| failure())? != buffer.len() {
        return Err(failure());
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let mut reply = [0u8; 4096];
        match socket.recv(&mut &mut reply[..], 0) {
            Ok(n) => {
                let response = NetlinkMessage::<RouteNetlinkMessage>::deserialize(&reply[..n])
                    .map_err(|_| failure())?;
                return match response.payload {
                    NetlinkPayload::Error(error)
                        if error.code.is_none()
                            || error.code.is_some_and(|c| matches!(c.get(), -2 | -19)) =>
                    {
                        Ok(())
                    }
                    _ => Err(failure()),
                };
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(_) => return Err(failure()),
        }
    }
}
