use crate::{machine::*, session_dir::create_session_dir};
use galleria_backend::{net::*, *};
use galleria_ipc::*;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::Instant,
};
pub type BackendFactory = Arc<
    dyn Fn(&BackendParams, &ConnectOptions) -> Result<Box<dyn Backend>, RpcError> + Send + Sync,
>;
pub trait NetControl: Send + Sync + 'static {
    fn recover(&self) -> Result<(), SessionError>;
    fn apply(&self, cfg: &NetConfig) -> Result<(), SessionError>;
    fn teardown(&self) -> Result<(), SessionError>;
}
#[derive(Clone)]
pub struct ManagerHandle {
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<Event>,
}
enum Command {
    Recover(oneshot::Sender<Result<(), SessionError>>),
    Connect(
        Box<ConnectParams>,
        u32,
        oneshot::Sender<Result<ConnectResult, RpcError>>,
    ),
    Disconnect(u32, oneshot::Sender<Status>),
    Status(oneshot::Sender<Status>),
    ProvideInput(ProvideInputParams, oneshot::Sender<Result<(), RpcError>>),
}
impl ManagerHandle {
    pub async fn connect(
        &self,
        params: ConnectParams,
        uid: u32,
    ) -> Result<ConnectResult, RpcError> {
        let (tx, rx) = oneshot::channel();
        self.commands
            .send(Command::Connect(Box::new(params), uid, tx))
            .await
            .map_err(|_| closed())?;
        rx.await.map_err(|_| closed())?
    }
    pub async fn disconnect(&self, uid: u32) -> Status {
        let (tx, rx) = oneshot::channel();
        if self
            .commands
            .send(Command::Disconnect(uid, tx))
            .await
            .is_err()
        {
            return Status::default();
        }
        rx.await.unwrap_or_default()
    }
    pub async fn status(&self) -> Status {
        let (tx, rx) = oneshot::channel();
        if self.commands.send(Command::Status(tx)).await.is_err() {
            return Status::default();
        }
        rx.await.unwrap_or_default()
    }
    pub async fn provide_input(&self, params: ProvideInputParams) -> Result<(), RpcError> {
        let (tx, rx) = oneshot::channel();
        self.commands
            .send(Command::ProvideInput(params, tx))
            .await
            .map_err(|_| closed())?;
        rx.await.map_err(|_| closed())?
    }
    pub(crate) async fn recover(&self) -> Result<(), SessionError> {
        let (tx, rx) = oneshot::channel();
        self.commands
            .send(Command::Recover(tx))
            .await
            .map_err(|_| SessionError::new(SessionErrorCode::BackendExited))?;
        rx.await
            .map_err(|_| SessionError::new(SessionErrorCode::BackendExited))?
    }
    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }
}
fn closed() -> RpcError {
    ErrorCode::Internal.to_rpc("Manager unavailable")
}
struct Active {
    handle: SessionHandle,
    dir: PathBuf,
}
struct Pending {
    params: Box<ConnectParams>,
    backend: Box<dyn Backend>,
    reply: oneshot::Sender<Result<ConnectResult, RpcError>>,
}
struct Manager {
    factory: BackendFactory,
    net: Arc<dyn NetControl>,
    policy: RetryPolicy,
    runtime_dir: PathBuf,
    phase: Phase,
    status: Status,
    params: Option<Box<ConnectParams>>,
    active: Option<Active>,
    pending: Option<Pending>,
    events: broadcast::Sender<Event>,
    backend_events: mpsc::Receiver<BackendEvent>,
    retry: Option<Instant>,
    input_timer: Option<Instant>,
    stop_timer: Option<Instant>,
    exit: Option<ExitKind>,
    terminal: Option<SessionError>,
    disconnect_replies: Vec<oneshot::Sender<Status>>,
    network_applied: bool,
    network_dirty: bool,
    counter: u64,
}
pub fn spawn_manager(
    factory: BackendFactory,
    net: Arc<dyn NetControl>,
    policy: RetryPolicy,
    runtime_dir: PathBuf,
) -> ManagerHandle {
    let (commands, rx) = mpsc::channel(32);
    let (events, _) = broadcast::channel(256);
    let (_, backend_events) = mpsc::channel(1);
    let manager = Manager {
        factory,
        net,
        policy,
        runtime_dir,
        phase: Phase::Disconnected,
        status: Status::default(),
        params: None,
        active: None,
        pending: None,
        events: events.clone(),
        backend_events,
        retry: None,
        input_timer: None,
        stop_timer: None,
        exit: None,
        terminal: None,
        disconnect_replies: vec![],
        network_applied: false,
        network_dirty: false,
        counter: 0,
    };
    tokio::spawn(manager.run(rx));
    ManagerHandle { commands, events }
}
async fn timer(deadline: Option<Instant>) {
    match deadline {
        Some(t) => tokio::time::sleep_until(t).await,
        None => std::future::pending().await,
    }
}
impl Manager {
    fn publish(&mut self) {
        self.status.state = match &self.phase {
            Phase::Disconnected => State::Disconnected,
            Phase::Connecting { .. } => State::Connecting,
            Phase::NeedsInput { .. } => State::NeedsInput,
            Phase::Connected => State::Connected,
            Phase::Reconnecting { .. } => State::Reconnecting,
            Phase::Disconnecting { .. } => State::Disconnecting,
            Phase::Error(_) => State::Error,
        };
        match &self.phase {
            Phase::Connecting { attempt }
            | Phase::Reconnecting { attempt }
            | Phase::NeedsInput { attempt, .. } => self.status.attempt = *attempt,
            _ => {}
        }
        self.status.error = if let Phase::Error(e) = &self.phase {
            Some(e.clone())
        } else {
            None
        };
        if !matches!(self.phase, Phase::NeedsInput { .. }) {
            self.status.input_request = None;
            self.input_timer = None;
        }
        let _ = self.events.send(Event::State(self.status.clone()));
    }
    fn teardown(&mut self) -> Result<(), SessionError> {
        let result = self.net.teardown();
        self.network_dirty = result.is_err();
        self.network_applied = false;
        self.status.net = None;
        self.status.local_addresses.clear();
        self.status.remote = None;
        self.status.stats = None;
        result
    }
    fn cleanup_failed(&mut self, error: SessionError) {
        self.retry = None;
        self.input_timer = None;
        let error = SessionError::new(error.code);
        if self.active.is_some() {
            self.terminal = Some(error);
            self.phase = Phase::Disconnecting {
                then_connect: false,
            };
            self.stop();
        } else {
            self.terminal = None;
            self.phase = Phase::Error(error);
            self.clear_session();
            if let Some(pending) = self.pending.take() {
                let _ = pending
                    .reply
                    .send(Err(ErrorCode::Internal.to_rpc("Network cleanup failed")));
            }
        }
        self.publish();
    }
    fn clear_session(&mut self) {
        self.params = None;
        self.retry = None;
        self.input_timer = None;
        self.status.input_request = None;
        self.status.detail = None;
    }
    fn start_session(
        &mut self,
        params: Box<ConnectParams>,
        backend: Box<dyn Backend>,
    ) -> Result<ConnectResult, RpcError> {
        self.counter += 1;
        self.status = Status {
            session: Some(self.counter),
            profile: Some(params.profile.clone()),
            protocol: Some(params.backend.protocol()),
            attempt: 1,
            ..Status::default()
        };
        self.policy.reconnect_enabled = params.options.reconnect.enabled;
        self.params = Some(params);
        self.terminal = None;
        self.phase = Phase::Connecting { attempt: 1 };
        if let Err(error) = self.spawn_backend(Some(backend)) {
            self.phase = Phase::Error(SessionError::new(SessionErrorCode::TunnelSetupFailed));
            self.clear_session();
            self.publish();
            return Err(error);
        }
        self.publish();
        Ok(ConnectResult {
            session: self.counter,
            status: self.status.clone(),
        })
    }
    fn spawn_backend(&mut self, prepared: Option<Box<dyn Backend>>) -> Result<(), RpcError> {
        if self.active.is_some() {
            return Err(closed());
        }
        let params = self.params.as_mut().ok_or_else(closed)?;
        let backend = match prepared {
            Some(b) => b,
            None => (self.factory)(&params.backend, &params.options)?,
        };
        let dir = create_session_dir(&self.runtime_dir)
            .map_err(|_| ErrorCode::Internal.to_rpc("Cannot create session directory"))?;
        let (events, rx) = mpsc::channel(64);
        self.backend_events = rx;
        self.exit = None;
        self.network_applied = false;
        self.active = Some(Active {
            handle: backend.spawn(SessionContext {
                session: self.counter,
                session_dir: dir.clone(),
                events,
            }),
            dir,
        });
        // An OTP belongs to one attempt. Retries must request a fresh value.
        if let BackendParams::Openvpn(p) = &mut params.backend {
            p.otp = None;
        }
        Ok(())
    }
    async fn transition(&mut self, input: Input) {
        let (next, effects) = step(&self.phase, input, &self.policy);
        self.phase = next;
        if let Phase::Error(error) = &self.phase {
            if self.active.is_some() {
                self.terminal = Some(error.clone());
                self.phase = Phase::Disconnecting {
                    then_connect: false,
                };
            } else {
                self.clear_session();
            }
        }
        for effect in effects {
            match effect {
                Effect::TeardownNet => {
                    if let Err(error) = self.teardown() {
                        self.cleanup_failed(error);
                        return;
                    }
                }
                Effect::StopBackend => self.stop(),
                Effect::SpawnBackend => {
                    if self.spawn_backend(None).is_err() {
                        self.phase =
                            Phase::Error(SessionError::new(SessionErrorCode::TunnelSetupFailed));
                        self.clear_session();
                        self.publish();
                    }
                }
                Effect::StartRetryTimer(delay) => {
                    self.retry = Some(Instant::now() + delay);
                }
                Effect::StartInputTimer(delay) => {
                    self.input_timer = Some(Instant::now() + delay);
                }
                Effect::Publish => self.publish(),
                Effect::ForwardInput | Effect::Reject(_) => {}
            }
        }
    }
    fn stop(&mut self) {
        self.retry = None;
        self.input_timer = None;
        if let Some(active) = &self.active {
            let _ = active.handle.commands.try_send(BackendCommand::Stop);
            if self.stop_timer.is_none() {
                self.stop_timer = Some(Instant::now() + Duration::from_secs(15));
            }
        }
    }
    async fn finish(&mut self) {
        if let Some(active) = self.active.take() {
            let _ = std::fs::remove_dir_all(active.dir);
        }
        self.stop_timer = None;
        // A terminal backend error does not make a clean replacement fail.
        if self.pending.is_some() && !self.network_dirty {
            self.terminal = None;
        }
        if let Some(error) = self.terminal.take() {
            self.phase = Phase::Error(error);
            self.clear_session();
            self.publish();
            if let Some(pending) = self.pending.take() {
                let _ = pending.reply.send(Err(
                    ErrorCode::Internal.to_rpc("Previous session teardown failed")
                ));
            }
        } else if matches!(self.phase, Phase::Disconnecting { .. }) {
            self.clear_session();
            if let Some(pending) = self.pending.take() {
                let result = self.start_session(pending.params, pending.backend);
                let _ = pending.reply.send(result);
            } else {
                self.phase = Phase::Disconnected;
                self.status = Status::default();
                self.publish();
            }
        } else {
            let kind = self
                .exit
                .take()
                .unwrap_or(ExitKind::Fatal(SessionErrorCode::BackendExited));
            self.transition(Input::Exited(kind)).await;
        }
        let status = self.status.clone();
        for reply in self.disconnect_replies.drain(..) {
            let _ = reply.send(status.clone());
        }
    }
    async fn command(&mut self, command: Command) {
        match command {
            Command::Recover(reply) => {
                let result = if matches!(self.phase, Phase::Disconnected | Phase::Error(_))
                    && self.active.is_none()
                {
                    self.net.recover()
                } else {
                    Err(SessionError::new(SessionErrorCode::NetSetupFailed))
                };
                let _ = reply.send(result);
            }
            Command::Status(reply) => {
                let _ = reply.send(self.status.clone());
            }
            Command::Connect(params, _uid, reply) => {
                let is_active = !matches!(self.phase, Phase::Disconnected | Phase::Error(_));
                if is_active && (!params.replace || self.pending.is_some()) {
                    let mut e = ErrorCode::AlreadyActive.to_rpc("A session is already active");
                    e.data = Some(
                        serde_json::json!({"code":"already_active","profile":self.status.profile}),
                    );
                    let _ = reply.send(Err(e));
                    return;
                }
                if self.active.is_none()
                    && self.network_dirty
                    && let Err(error) = self.teardown()
                {
                    self.cleanup_failed(error);
                    let _ = reply.send(Err(ErrorCode::Internal.to_rpc("Network cleanup failed")));
                    return;
                }
                let backend = match (self.factory)(&params.backend, &params.options) {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = reply.send(Err(e));
                        return;
                    }
                };
                if is_active {
                    self.pending = Some(Pending {
                        params,
                        backend,
                        reply,
                    });
                    self.transition(Input::Connect { replace: true }).await;
                    if self.active.is_none() {
                        self.finish().await;
                    }
                } else {
                    let result = self.start_session(params, backend);
                    if result.is_err() {
                        self.phase =
                            Phase::Error(SessionError::new(SessionErrorCode::TunnelSetupFailed));
                        self.clear_session();
                        self.publish();
                    }
                    let _ = reply.send(result);
                }
            }
            Command::Disconnect(_uid, reply) => {
                if let Some(pending) = self.pending.take() {
                    let _ = pending
                        .reply
                        .send(Err(ErrorCode::NotActive.to_rpc("Connection cancelled")));
                }
                if self.active.is_none()
                    && self.network_dirty
                    && let Err(error) = self.teardown()
                {
                    self.cleanup_failed(error);
                    let _ = reply.send(self.status.clone());
                    return;
                }
                self.disconnect_replies.push(reply);
                self.terminal = None;
                self.transition(Input::Disconnect).await;
                if self.active.is_none() {
                    if !matches!(self.phase, Phase::Error(_)) {
                        self.phase = Phase::Disconnected;
                        self.status = Status::default();
                        self.clear_session();
                        self.publish();
                    }
                    let s = self.status.clone();
                    for reply in self.disconnect_replies.drain(..) {
                        let _ = reply.send(s.clone());
                    }
                }
            }
            Command::ProvideInput(params, reply) => {
                let error = if self.params.is_none() {
                    Some(ErrorCode::NotActive)
                } else if !matches!(self.phase, Phase::NeedsInput { .. })
                    || self.status.session != Some(params.session)
                    || self.status.input_request.as_ref().map(|r| r.request) != Some(params.request)
                {
                    Some(ErrorCode::InputNotExpected)
                } else {
                    None
                };
                if let Some(code) = error {
                    let _ = reply.send(Err(code.to_rpc("Input not expected")));
                    return;
                }
                let result = if let Some(active) = &self.active {
                    active
                        .handle
                        .commands
                        .try_send(BackendCommand::ProvideInput(params.values))
                        .map_err(|_| closed())
                } else {
                    Err(closed())
                };
                if result.is_ok() {
                    self.transition(Input::InputProvided).await;
                }
                let _ = reply.send(result);
            }
        }
    }
    async fn backend_event(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::NetDown { reply } => {
                let result = self.teardown();
                let _ = reply.send(());
                match result {
                    Ok(()) => self.publish(),
                    Err(error) => self.cleanup_failed(error),
                }
            }
            BackendEvent::NetUp { config, reply } => {
                if matches!(
                    self.phase,
                    Phase::Disconnecting { .. } | Phase::Error(_) | Phase::Disconnected
                ) {
                    let _ = reply.send(Err("Session is stopping".into()));
                    return;
                }
                let result = if self.network_applied || self.network_dirty {
                    self.teardown()
                } else {
                    Ok(())
                }
                .and_then(|()| {
                    self.network_dirty = true;
                    self.net.apply(&config)
                });
                match result {
                    Ok(()) => {
                        self.network_applied = true;
                        self.status.net = Some(NetInfo {
                            interface: config.interface,
                            dns_servers: config.dns_servers,
                            search_domains: config.search_domains,
                            full_tunnel: is_full_tunnel_v4(&config.routes),
                            routes: config.routes.iter().map(ToString::to_string).collect(),
                            ipv6_block: match config.ipv6_block {
                                Ipv6Block::None => Ipv6BlockInfo::None,
                                Ipv6Block::Unreachable => Ipv6BlockInfo::Unreachable,
                            },
                        });
                        let _ = reply.send(Ok(()));
                        self.publish();
                    }
                    Err(e) => {
                        let _ = reply.send(Err(SessionError::new(e.code).message));
                        self.transition(Input::NetFailed(e.code)).await;
                    }
                }
            }
            BackendEvent::Up(info) => {
                if self.network_applied
                    && matches!(
                        self.phase,
                        Phase::Connecting { .. } | Phase::Reconnecting { .. }
                    )
                {
                    self.status.local_addresses = info.local_addresses;
                    self.status.remote = info.remote;
                    self.transition(Input::Up).await;
                }
            }
            BackendEvent::Restarting(_) => {
                self.transition(Input::Restarting).await;
            }
            BackendEvent::InputRequired(request) => {
                if matches!(
                    self.phase,
                    Phase::Connecting { .. } | Phase::Reconnecting { .. }
                ) {
                    self.status.input_request = Some(InputRequest {
                        expires_in_secs: 120,
                        ..request
                    });
                    self.transition(Input::InputRequired).await;
                }
            }
            BackendEvent::Progress(detail) => {
                if matches!(
                    self.phase,
                    Phase::Connecting { .. } | Phase::Reconnecting { .. }
                ) {
                    self.status.detail = Some(detail);
                    self.publish();
                }
            }
            BackendEvent::Stats(stats) => {
                if self.status.state == State::Connected {
                    self.status.stats = Some(stats.clone());
                    let _ = self.events.send(Event::Stats(stats));
                }
            }
            BackendEvent::Log(_) => {} // Central redaction and log buffer arrive in Task 5.
            BackendEvent::Exited(reason) => {
                self.exit = Some(match reason {
                    ExitReason::Stopped => ExitKind::Stopped,
                    ExitReason::AuthFailed(_) => ExitKind::Auth,
                    ExitReason::Transport(_) => ExitKind::Transport,
                    ExitReason::Fatal(e) => ExitKind::Fatal(e.code),
                });
            }
        }
    }
    async fn run(mut self, mut rx: mpsc::Receiver<Command>) {
        loop {
            tokio::select! {
             command=rx.recv()=>match command{Some(command)=>self.command(command).await,None=>{let _=self.teardown();if let Some(active)=self.active.take(){active.handle.task.abort();let _=active.handle.task.await;let _=std::fs::remove_dir_all(active.dir);}break;}},
             event=self.backend_events.recv(),if self.active.is_some()=>{if let Some(event)=event{self.backend_event(event).await;}else{tokio::task::yield_now().await;}},
             _=async{match &mut self.active{Some(active)=>{let _=(&mut active.handle.task).await;},None=>std::future::pending().await}}=>{
              // The task may complete while Exited is still queued. Drain before interpreting its exit.
              while let Ok(event)=self.backend_events.try_recv(){self.backend_event(event).await;}
              self.finish().await;
             },
             _=timer(self.retry)=>{self.retry=None;self.transition(Input::RetryTimer).await;},
             _=timer(self.input_timer)=>{self.input_timer=None;self.transition(Input::InputTimeout).await;},
             _=timer(self.stop_timer)=>{if let Some(active)=&self.active{active.handle.task.abort();}self.stop_timer=None;},
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use galleria_backend::fake::FakeBackend;
    use std::sync::atomic::AtomicUsize;
    struct Net;
    impl NetControl for Net {
        fn recover(&self) -> Result<(), SessionError> {
            Ok(())
        }
        fn apply(&self, _: &NetConfig) -> Result<(), SessionError> {
            Ok(())
        }
        fn teardown(&self) -> Result<(), SessionError> {
            Ok(())
        }
    }
    #[tokio::test]
    async fn secrets_are_dropped_at_session_end() {
        let tmp = tempfile::tempdir().unwrap();
        let (events, _) = broadcast::channel(16);
        let (_, backend_events) = mpsc::channel(1);
        let factory: BackendFactory = Arc::new(|_, _| {
            Ok(Box::new(FakeBackend::new(
                vec![],
                Arc::new(AtomicUsize::new(0)),
            )))
        });
        let mut manager = Manager {
            factory: factory.clone(),
            net: Arc::new(Net),
            policy: RetryPolicy::default(),
            runtime_dir: tmp.path().to_path_buf(),
            phase: Phase::Disconnected,
            status: Status::default(),
            params: None,
            active: None,
            pending: None,
            events,
            backend_events,
            retry: None,
            input_timer: None,
            stop_timer: None,
            exit: None,
            terminal: None,
            disconnect_replies: vec![],
            network_applied: false,
            network_dirty: false,
            counter: 0,
        };
        let params:ConnectParams=serde_json::from_value(serde_json::json!({"profile":{"id":"test","name":"test"},"replace":false,"backend":{"protocol":"openvpn","config":"client","secret_blocks":{},"username":null,"password":"synthetic","otp":"once","key_passphrase":null},"options":{"ipv6_block":true,"ignore_pushed_routes":false,"reconnect":{"enabled":true}}})).unwrap();
        let backend = factory(&params.backend, &params.options).unwrap();
        manager.start_session(Box::new(params), backend).unwrap();
        assert!(manager.params.is_some());
        if let BackendParams::Openvpn(p) = &manager.params.as_ref().unwrap().backend {
            assert!(p.otp.is_none());
        }
        manager.transition(Input::Disconnect).await;
        let event = manager.backend_events.recv().await.unwrap();
        manager.backend_event(event).await;
        let active = manager.active.as_mut().unwrap();
        (&mut active.handle.task).await.unwrap();
        while let Ok(event) = manager.backend_events.try_recv() {
            manager.backend_event(event).await;
        }
        manager.finish().await;
        assert!(manager.params.is_none());
        assert!(manager.pending.is_none());
        assert!(manager.active.is_none());
    }
}
