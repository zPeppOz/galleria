#![allow(dead_code)]
use galleria_backend::{fake::*, net::*, *};
use galleria_ipc::*;
use galleriad::{machine::RetryPolicy, manager::*};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
pub fn params(replace: bool) -> ConnectParams {
    serde_json::from_value(serde_json::json!({"profile":{"id":"test","name":"Test VPN"},"replace":replace,"backend":{"protocol":"wireguard","private_key":"synthetic","addresses":[],"dns":[],"search_domains":[],"mtu":null,"peers":[]},"options":{"ipv6_block":true,"ignore_pushed_routes":false,"reconnect":{"enabled":true}}})).unwrap()
}
pub fn config(ip: &str) -> NetConfig {
    NetConfig {
        interface: "glr-wg0".into(),
        dns_servers: vec![ip.parse().unwrap()],
        search_domains: vec![],
        routes: vec!["0.0.0.0/0".parse().unwrap()],
        install_routes: Some(51820),
        ipv6_block: Ipv6Block::Unreachable,
    }
}
pub fn up() -> FakeStep {
    FakeStep::Emit(FakeEvent::Up(TunnelInfo {
        local_addresses: vec!["10.0.0.2".parse().unwrap()],
        remote: None,
    }))
}
pub fn online() -> Vec<FakeStep> {
    vec![FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.1"))), up()]
}
#[derive(Default)]
pub struct Net {
    pub log: Mutex<Vec<String>>,
    pub applied: AtomicBool,
    pub refuse: AtomicBool,
    pub fail_teardown: AtomicBool,
}
impl NetControl for Net {
    fn recover(&self) -> Result<(), SessionError> {
        self.log.lock().unwrap().push("recover".into());
        Ok(())
    }
    fn apply(&self, cfg: &NetConfig) -> Result<(), SessionError> {
        assert!(
            !self.applied.swap(true, Ordering::SeqCst),
            "overlapping networks"
        );
        self.log
            .lock()
            .unwrap()
            .push(format!("apply({})", cfg.dns_servers[0]));
        if self.refuse.load(Ordering::SeqCst) {
            Err(SessionError::new(SessionErrorCode::NetSetupFailed))
        } else {
            Ok(())
        }
    }
    fn teardown(&self) -> Result<(), SessionError> {
        if self.fail_teardown.load(Ordering::SeqCst) {
            self.log.lock().unwrap().push("teardown_failed".into());
            return Err(SessionError::new(SessionErrorCode::NetSetupFailed));
        }
        self.applied.store(false, Ordering::SeqCst);
        self.log.lock().unwrap().push("teardown".into());
        Ok(())
    }
}
pub struct Harness {
    pub handle: ManagerHandle,
    pub net: Arc<Net>,
    pub live: Arc<AtomicUsize>,
    pub spawns: Arc<AtomicUsize>,
    pub dirs: Arc<Mutex<Vec<PathBuf>>>,
    pub tmp: tempfile::TempDir,
}
impl Harness {
    pub fn new(scripts: Vec<Vec<FakeStep>>) -> Self {
        Self::with_policy(scripts, RetryPolicy::default())
    }
    pub fn with_policy(scripts: Vec<Vec<FakeStep>>, policy: RetryPolicy) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let net = Arc::new(Net::default());
        let live = Arc::new(AtomicUsize::new(0));
        let spawns = Arc::new(AtomicUsize::new(0));
        let dirs = Arc::new(Mutex::new(Vec::new()));
        let queue = Mutex::new(VecDeque::from(scripts));
        let l = live.clone();
        let s = spawns.clone();
        let d = dirs.clone();
        let n = net.clone();
        let factory: BackendFactory = Arc::new(move |_, _| {
            s.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(Observed {
                fake: FakeBackend::new(
                    queue.lock().unwrap().pop_front().unwrap_or_default(),
                    l.clone(),
                ),
                dirs: d.clone(),
                net: n.clone(),
                live: l.clone(),
            }))
        });
        let handle = spawn_manager(factory, net.clone(), policy, tmp.path().to_path_buf());
        Self {
            handle,
            net,
            live,
            spawns,
            dirs,
            tmp,
        }
    }
    pub async fn state(&self, state: State) -> Status {
        wait_state(&self.handle, state).await
    }
}
struct Observed {
    live: Arc<AtomicUsize>,
    fake: FakeBackend,
    dirs: Arc<Mutex<Vec<PathBuf>>>,
    net: Arc<Net>,
}
impl Backend for Observed {
    fn protocol(&self) -> Protocol {
        self.fake.protocol()
    }
    fn capabilities(&self) -> Capabilities {
        self.fake.capabilities()
    }
    fn spawn(self: Box<Self>, ctx: SessionContext) -> SessionHandle {
        assert_eq!(
            self.live.load(Ordering::SeqCst),
            0,
            "second backend spawned before exit"
        );
        self.dirs.lock().unwrap().push(ctx.session_dir.clone());
        self.net
            .log
            .lock()
            .unwrap()
            .push(format!("spawn({})", ctx.session));
        let handle = Box::new(self.fake).spawn(ctx);
        let (commands, mut rx) = tokio::sync::mpsc::channel(16);
        let net = self.net;
        let task = tokio::spawn(async move {
            let mut inner = handle.task;
            loop {
                tokio::select! {
                 command=rx.recv()=>{if let Some(command)=command {if matches!(command,BackendCommand::Stop){net.log.lock().unwrap().push("stop".into());}let _=handle.commands.send(command).await;}}
                 _=&mut inner=>{net.log.lock().unwrap().push("exit".into());break;}
                }
            }
        });
        SessionHandle { commands, task }
    }
}
pub async fn wait_state(handle: &ManagerHandle, state: State) -> Status {
    for _ in 0..300 {
        let s = handle.status().await;
        if s.state == state {
            return s;
        }
        tokio::task::yield_now().await;
    }
    panic!("state {:?}, wanted {:?}", handle.status().await, state)
}
