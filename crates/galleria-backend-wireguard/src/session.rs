use crate::{validate::ValidWireGuard, *};
use galleria_backend::{
    net::{carries_ipv6_default, is_full_tunnel_v4},
    *,
};
use galleria_ipc::*;
use std::time::{Duration, UNIX_EPOCH};
use tokio::{
    sync::{mpsc, oneshot},
    time::{Instant, MissedTickBehavior},
};
pub struct WireGuardBackend {
    cfg: ValidWireGuard,
    ipv6_block: bool,
    device: Box<dyn WgDevice>,
}
impl WireGuardBackend {
    pub fn new(cfg: ValidWireGuard, options: &ConnectOptions, device: Box<dyn WgDevice>) -> Self {
        Self {
            cfg,
            ipv6_block: options.ipv6_block,
            device,
        }
    }
}
pub fn capabilities() -> Capabilities {
    Capabilities {
        handshake: true,
        interactive_auth: false,
        server_pushed_config: false,
    }
}
impl Backend for WireGuardBackend {
    fn protocol(&self) -> Protocol {
        Protocol::Wireguard
    }
    fn capabilities(&self) -> Capabilities {
        capabilities()
    }
    fn spawn(self: Box<Self>, ctx: SessionContext) -> SessionHandle {
        let (commands, mut rx) = mpsc::channel(16);
        let runtime = tokio::runtime::Handle::current();
        // Netlink and the adapter's DNS cleanup are blocking. They never occupy an async executor thread.
        let task = tokio::task::spawn_blocking(move || {
            runtime.block_on(async move {
                let mut this = *self;
                let mut created = false;
                let mut reason = this.run(&ctx, &mut rx, &mut created).await;
                if created && this.device.remove().is_err() {
                    reason = fatal(SessionErrorCode::TunnelSetupFailed);
                }
                let _ = ctx.events.send(BackendEvent::Exited(reason)).await;
            })
        });
        SessionHandle { commands, task }
    }
}
fn fatal(code: SessionErrorCode) -> ExitReason {
    ExitReason::Fatal(SessionError::new(code))
}
impl WireGuardBackend {
    async fn configuration(&self) -> Result<DeviceConfig, ExitReason> {
        let mut peers = Vec::new();
        for p in &self.cfg.peers {
            let endpoint = match &p.endpoint {
                None => None,
                Some(name) => Some(
                    tokio::time::timeout(Duration::from_secs(5), async {
                        tokio::net::lookup_host(name)
                            .await
                            .ok()
                            .and_then(|mut addresses| addresses.next())
                    })
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| ExitReason::Transport("Endpoint resolution failed".into()))?,
                ),
            };
            peers.push(DevicePeer {
                public_key: p.public_key,
                preshared_key: p.preshared_key.clone(),
                endpoint,
                allowed_ips: p.allowed_ips.clone(),
                persistent_keepalive: p.persistent_keepalive,
            });
        }
        let net = net_config(&self.cfg, self.ipv6_block);
        Ok(DeviceConfig {
            private_key: self.cfg.private_key.clone(),
            addresses: self.cfg.addresses.clone(),
            mtu: self.cfg.mtu,
            peers,
            fwmark: (is_full_tunnel_v4(&net.routes) || carries_ipv6_default(&net.routes))
                .then_some(WG_FWMARK),
        })
    }
    async fn run(
        &mut self,
        ctx: &SessionContext,
        rx: &mut mpsc::Receiver<BackendCommand>,
        created: &mut bool,
    ) -> ExitReason {
        let config = tokio::select! {
            result=self.configuration()=>match result {Ok(cfg)=>cfg,Err(reason)=>return reason},
            _=stopping(rx)=>return ExitReason::Stopped,
        };
        *created = true;
        if self.device.create().is_err() || self.device.configure(&config).is_err() {
            return fatal(SessionErrorCode::TunnelSetupFailed);
        }
        let net = net_config(&self.cfg, self.ipv6_block);
        let (reply, receive) = oneshot::channel();
        if ctx
            .events
            .send(BackendEvent::NetUp { config: net, reply })
            .await
            .is_err()
        {
            return ExitReason::Stopped;
        }
        let accepted = tokio::select! { result=receive=>matches!(result,Ok(Ok(()))), _=stopping(rx)=>return ExitReason::Stopped };
        if !accepted {
            return fatal(SessionErrorCode::NetSetupFailed);
        }
        let start = Instant::now();
        if ctx
            .events
            .send(BackendEvent::Up(TunnelInfo {
                local_addresses: self.cfg.addresses.iter().map(|n| n.addr()).collect(),
                remote: config.peers.first().and_then(|p| p.endpoint),
            }))
            .await
            .is_err()
        {
            return ExitReason::Stopped;
        }
        let mut interval =
            tokio::time::interval_at(start + Duration::from_secs(1), Duration::from_secs(1));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut handshake = false;
        let mut warned = false;
        let keepalive = self
            .cfg
            .peers
            .iter()
            .any(|p| p.persistent_keepalive.is_some_and(|n| n != 0));
        loop {
            tokio::select! {
                biased;
                _=stopping(rx)=>return ExitReason::Stopped,
                _=interval.tick()=>{
                    let stats=match self.device.read() {Ok(s)=>s,Err(_)=>return ExitReason::Transport("Cannot read WireGuard statistics".into())};
                    handshake|=stats.last_handshake.is_some();
                    if !keepalive && !handshake && !warned && start.elapsed()>=Duration::from_secs(10) {
                        warned=true;
                        let _=ctx.events.send(BackendEvent::Log(LogLine {level:LogLevel::Warn,message:"no handshake yet".into()})).await;
                    }
                    let event=Stats {rx_bytes:stats.rx_bytes,tx_bytes:stats.tx_bytes,last_handshake:stats.last_handshake.and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs().to_string())};
                    if ctx.events.send(BackendEvent::Stats(event)).await.is_err() {return ExitReason::Stopped;}
                }
            }
        }
    }
}
async fn stopping(rx: &mut mpsc::Receiver<BackendCommand>) {
    while let Some(cmd) = rx.recv().await {
        if matches!(cmd, BackendCommand::Stop) {
            return;
        }
    }
}
