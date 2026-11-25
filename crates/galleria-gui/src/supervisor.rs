//! Keeps one long connection to the daemon for events and relinks on its own (spec §3.3, D27).
//! Pure Rust and tokio: the app framework is not imported here; the webview side is an [`EventSink`].

use crate::link::{self, DaemonView, LinkFailure};
use galleria_ipc::{Event, LogEntry, Stats, Status, SubscribeParams, client::Client};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Notify, mpsc, oneshot, watch},
    task::JoinHandle,
};

/// Where the supervisor delivers what it learns. Calls come from one thread of their own, in order, so a slow
/// implementation never holds up the connection to the daemon; it only delays what it receives.
pub trait EventSink: Send + Sync + 'static {
    fn link(&self, view: &DaemonView);
    fn state(&self, status: &Status);
    fn stats(&self, stats: &Stats);
    fn log(&self, entry: &LogEntry);
}

/// Waits between attempts to reach a daemon that is not there; the last value repeats.
#[derive(Debug, Clone)]
pub struct RelinkPolicy {
    pub delays: Vec<Duration>,
}

impl Default for RelinkPolicy {
    fn default() -> Self {
        Self {
            delays: vec![
                Duration::from_millis(500),
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(5),
            ],
        }
    }
}

impl RelinkPolicy {
    /// The wait after `failures` consecutive failed attempts (0 after the first).
    fn delay(&self, failures: usize) -> Duration {
        self.delays
            .get(failures)
            .or(self.delays.last())
            .copied()
            .unwrap_or(Duration::from_secs(5))
    }
}

/// The running supervisor. Dropping it without [`SupervisorHandle::shutdown`] also stops the task.
pub struct SupervisorHandle {
    view: watch::Receiver<DaemonView>,
    retry: Arc<Notify>,
    attempts: Arc<AtomicUsize>,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl SupervisorHandle {
    /// The last snapshot of the link, kept up to date by the supervisor itself.
    pub fn view(&self) -> DaemonView {
        self.view.borrow().clone()
    }

    /// Skips the wait in progress and tries to reach the daemon now.
    pub fn retry_now(&self) {
        self.retry.notify_one();
    }

    /// How many connection attempts have started so far.
    pub fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }

    /// Closes the connection to the daemon and waits for the task to end.
    pub async fn shutdown(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}

/// Starts the supervisor on the current tokio runtime. The first call to `sink.link` is `connecting`.
pub fn spawn(
    socket: PathBuf,
    client_version: String,
    sink: Arc<dyn EventSink>,
    policy: RelinkPolicy,
) -> SupervisorHandle {
    let (view_tx, view_rx) = watch::channel(DaemonView::connecting());
    let (stop, stop_rx) = oneshot::channel();
    let retry = Arc::new(Notify::new());
    let attempts = Arc::new(AtomicUsize::new(0));
    let outbox = Outbox::start(sink);
    outbox.send(Out::Link(DaemonView::connecting()));
    let task = tokio::spawn(
        Supervisor {
            socket,
            client_version,
            policy,
            outbox,
            view: view_tx,
            retry: retry.clone(),
            attempts: attempts.clone(),
            stop: stop_rx,
        }
        .run(),
    );
    SupervisorHandle {
        view: view_rx,
        retry,
        attempts,
        stop,
        task,
    }
}

enum Out {
    Link(DaemonView),
    State(Status),
    Stats(Stats),
    Log(LogEntry),
}

/// Hands everything to the sink from a thread of its own, through an unbounded queue, so that the task reading
/// the daemon never waits for the sink.
struct Outbox {
    queue: mpsc::UnboundedSender<Out>,
}

impl Outbox {
    fn start(sink: Arc<dyn EventSink>) -> Self {
        let (queue, mut rx) = mpsc::unbounded_channel();
        let thread = std::thread::Builder::new()
            .name("galleria-sink".into())
            .spawn(move || {
                while let Some(out) = rx.blocking_recv() {
                    match out {
                        Out::Link(view) => sink.link(&view),
                        Out::State(status) => sink.state(&status),
                        Out::Stats(stats) => sink.stats(&stats),
                        Out::Log(entry) => sink.log(&entry),
                    }
                }
            });
        if let Err(e) = thread {
            eprintln!("galleria-gui: cannot start the event delivery thread: {e}");
        }
        Self { queue }
    }

    fn send(&self, out: Out) {
        // The thread is gone only if the sink panicked; there is nobody left to tell.
        let _ = self.queue.send(out);
    }
}

struct Supervisor {
    socket: PathBuf,
    client_version: String,
    policy: RelinkPolicy,
    outbox: Outbox,
    view: watch::Sender<DaemonView>,
    retry: Arc<Notify>,
    attempts: Arc<AtomicUsize>,
    stop: oneshot::Receiver<()>,
}

impl Supervisor {
    async fn run(mut self) {
        let mut failures = 0usize;
        loop {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            let outcome = tokio::select! {
                _ = &mut self.stop => return,
                outcome = link::attempt(&self.socket, &self.client_version) => outcome,
            };
            let failure = match outcome {
                Ok((mut client, hello)) => {
                    failures = 0;
                    match subscribe(&mut client).await {
                        Ok(()) => {
                            self.publish(DaemonView::connected(&hello));
                            if !self.forward_events(&mut client).await {
                                return;
                            }
                            // The connection ended: look for the daemon again right away.
                            continue;
                        }
                        Err(failure) => failure,
                    }
                }
                Err(failure) => failure,
            };
            self.publish_failure(&failure);
            let delay = self.policy.delay(failures);
            failures += 1;
            if !self.wait(delay).await {
                return;
            }
        }
    }

    /// Reads events until the connection ends (`true`) or a shutdown is requested (`false`).
    async fn forward_events(&mut self, client: &mut Client) -> bool {
        loop {
            let event = tokio::select! {
                _ = &mut self.stop => return false,
                event = client.next_event() => event,
            };
            match event {
                None => return true,
                Some(Event::State(status)) => {
                    self.view
                        .send_modify(|view| view.status = Some(status.clone()));
                    self.outbox.send(Out::State(status));
                }
                Some(Event::Stats(stats)) => self.outbox.send(Out::Stats(stats)),
                Some(Event::Log(entry)) => self.outbox.send(Out::Log(entry)),
            }
        }
    }

    /// Waits for `delay` or an explicit retry (`true`), or until a shutdown is requested (`false`).
    async fn wait(&mut self, delay: Duration) -> bool {
        tokio::select! {
            _ = &mut self.stop => false,
            () = tokio::time::sleep(delay) => true,
            () = self.retry.notified() => {
                // Show that the attempt started: the next failure then changes the view again.
                self.publish(DaemonView::connecting());
                true
            }
        }
    }

    fn publish(&self, view: DaemonView) {
        self.view.send_replace(view.clone());
        self.outbox.send(Out::Link(view));
    }

    /// A failure is announced once: repeating the same failed attempt changes nothing for the webview.
    fn publish_failure(&self, failure: &LinkFailure) {
        let unchanged = {
            let current = self.view.borrow();
            current.link == failure.state && current.incompatible == failure.incompatible
        };
        if unchanged {
            return;
        }
        eprintln!(
            "galleria-gui: daemon link {:?}: {}",
            failure.state, failure.detail
        );
        self.publish(DaemonView::failed(failure));
    }
}

async fn subscribe(client: &mut Client) -> Result<(), LinkFailure> {
    let params = SubscribeParams {
        stats: true,
        logs: true,
    };
    client
        .call::<_, serde_json::Value>("subscribe", &params)
        .await
        .map(|_| ())
        .map_err(|e| link::classify(&e))
}
