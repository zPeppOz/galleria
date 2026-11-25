//! Test support: a real `galleriad` serving a fake backend on a temporary socket, and scripted servers for the cases
//! the real daemon never produces. Reduced copy of `crates/galleriad/tests/support/mod.rs` plus the startup of
//! `serve` from `crates/galleriad/tests/ipc_server.rs` (spec §14, Q17); the copied parts are not edited.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use galleria_backend::{fake::*, net::*, *};
use galleria_ipc::{client::Client, *};
use galleriad::{
    config::DaemonConfig,
    machine::RetryPolicy,
    manager::*,
    server::{DaemonInfo, serve},
};
use std::{
    collections::VecDeque,
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::sync::CancellationToken;

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
        let handle = spawn_manager(
            factory,
            net.clone(),
            RetryPolicy::default(),
            tmp.path().to_path_buf(),
        );
        Self {
            handle,
            net,
            live,
            spawns,
            dirs,
            tmp,
        }
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

/// The daemon information a fake daemon announces in `hello`: version 0.1.0 and the given backends.
pub fn info(backends: Vec<BackendInfo>) -> DaemonInfo {
    DaemonInfo {
        backend_probe: None,
        version: "0.1.0".into(),
        backends,
    }
}

/// A real daemon (`galleriad::server::serve`) with a fake backend, listening on a temporary socket.
pub struct FakeDaemon {
    pub socket: PathBuf,
    pub harness: Harness,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}
impl FakeDaemon {
    /// Starts a daemon with its socket in a directory of its own (the harness directory).
    pub async fn start(scripts: Vec<Vec<FakeStep>>, info: DaemonInfo) -> Self {
        let harness = Harness::new(scripts);
        let dir = harness.tmp.path().to_path_buf();
        Self::start_in(&dir, harness, info).await
    }
    /// Starts a daemon whose socket is `dir/galleriad.sock`; used to bring a daemon back on the same path.
    pub async fn start_in(dir: &Path, harness: Harness, info: DaemonInfo) -> Self {
        let socket = dir.join("galleriad.sock");
        let cfg = DaemonConfig {
            socket: socket.clone(),
            group: nix::unistd::Group::from_gid(nix::unistd::getgid())
                .unwrap()
                .unwrap()
                .name,
            runtime_dir: harness.tmp.path().to_path_buf(),
            hello_timeout: Duration::from_secs(5),
            max_clients: 16,
        };
        let shutdown = CancellationToken::new();
        let task = tokio::spawn(serve(cfg, harness.handle.clone(), info, shutdown.clone()));
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                assert!(!task.is_finished(), "server stopped");
                if let Ok(mut probe) = Client::connect(&socket).await
                    && probe.hello("readiness", "1").await.is_ok()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        Self {
            socket,
            harness,
            shutdown,
            task,
        }
    }
    /// A second client of the same daemon, already past `hello`.
    pub async fn client(&self) -> Client {
        let mut c = Client::connect(&self.socket).await.unwrap();
        c.hello("test", "1").await.unwrap();
        c
    }
    pub async fn stop(self) {
        self.shutdown.cancel();
        self.task.await.unwrap().unwrap();
        assert!(!self.socket.exists());
    }
}

/// A server that follows a script: every accepted connection goes to `handler`. For the cases the real daemon
/// never produces on demand (`incompatible_protocol`, a close before the reply).
pub struct Scripted {
    pub socket: PathBuf,
    task: tokio::task::JoinHandle<()>,
}
impl Scripted {
    pub fn listen<F, Fut>(dir: &Path, handler: F) -> Self
    where
        F: Fn(UnixStream) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let socket = dir.join("galleriad.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(handler(stream));
            }
        });
        Self { socket, task }
    }
}
impl Drop for Scripted {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.socket);
    }
}
