//! Scripted backend for tests; no network or filesystem I/O.
use crate::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
pub enum FakeStep {
    Emit(FakeEvent),
    WaitCommand,
    Sleep(Duration),
}
pub enum FakeEvent {
    Progress(String),
    InputRequired(InputRequest),
    NetUp(NetConfig),
    NetDown,
    Up(TunnelInfo),
    Restarting(String),
    Stats(Stats),
    Log(LogLine),
    Exited(ExitReason),
}
pub struct FakeBackend {
    script: Vec<FakeStep>,
    live: Arc<AtomicUsize>,
}
impl FakeBackend {
    pub fn new(script: Vec<FakeStep>, live: Arc<AtomicUsize>) -> Self {
        Self { script, live }
    }
}
struct Live(Arc<AtomicUsize>);
impl Drop for Live {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
async fn stop(events: &mpsc::Sender<BackendEvent>) {
    let (reply, rx) = oneshot::channel();
    if events.send(BackendEvent::NetDown { reply }).await.is_ok() {
        let _ = rx.await;
    }
    let _ = events.send(BackendEvent::Exited(ExitReason::Stopped)).await;
}
impl Backend for FakeBackend {
    fn protocol(&self) -> Protocol {
        Protocol::Wireguard
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            handshake: true,
            interactive_auth: true,
            server_pushed_config: true,
        }
    }
    fn spawn(self: Box<Self>, ctx: SessionContext) -> SessionHandle {
        let (commands, mut rx) = mpsc::channel(16);
        self.live.fetch_add(1, Ordering::SeqCst);
        let FakeBackend { script, live } = *self;
        let guard = Live(live);
        let task = tokio::spawn(async move {
            let _guard = guard;
            for step in script {
                let event = match step {
                    FakeStep::WaitCommand => match rx.recv().await {
                        Some(BackendCommand::ProvideInput(_)) => continue,
                        Some(BackendCommand::Stop) | None => {
                            stop(&ctx.events).await;
                            return;
                        }
                    },
                    FakeStep::Sleep(delay) => {
                        tokio::select! {_=tokio::time::sleep(delay)=>{},command=rx.recv()=>{if !matches!(command,Some(BackendCommand::ProvideInput(_))){stop(&ctx.events).await;return;}}}
                        continue;
                    }
                    FakeStep::Emit(e) => e,
                };
                let event = match event {
                    FakeEvent::NetUp(config) => {
                        let (reply, wait) = oneshot::channel();
                        if ctx
                            .events
                            .send(BackendEvent::NetUp { config, reply })
                            .await
                            .is_err()
                        {
                            return;
                        }
                        let _ = wait.await;
                        continue;
                    }
                    FakeEvent::NetDown => {
                        let (reply, wait) = oneshot::channel();
                        if ctx
                            .events
                            .send(BackendEvent::NetDown { reply })
                            .await
                            .is_err()
                        {
                            return;
                        }
                        let _ = wait.await;
                        continue;
                    }
                    FakeEvent::Progress(s) => BackendEvent::Progress(s),
                    FakeEvent::InputRequired(r) => BackendEvent::InputRequired(r),
                    FakeEvent::Up(t) => BackendEvent::Up(t),
                    FakeEvent::Restarting(s) => BackendEvent::Restarting(s),
                    FakeEvent::Stats(s) => BackendEvent::Stats(s),
                    FakeEvent::Log(l) => BackendEvent::Log(l),
                    FakeEvent::Exited(reason) => {
                        let _ = ctx.events.send(BackendEvent::Exited(reason)).await;
                        return;
                    }
                };
                if ctx.events.send(event).await.is_err() {
                    return;
                }
            }
            while let Some(command) = rx.recv().await {
                if matches!(command, BackendCommand::Stop) {
                    stop(&ctx.events).await;
                    return;
                }
            }
        });
        SessionHandle { commands, task }
    }
}
