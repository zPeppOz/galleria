//! A daemon for the WebDriver smoke test (lab/tests/gui-smoke.sh): the real `galleriad::server::serve` with a fake
//! backend, listening on `<dir>/galleriad.sock` until SIGTERM or SIGINT. It exists because the real `galleriad run`
//! cleans the network state when it starts (`ip rule del …`), which a smoke test must never do (S5, spec §17).
//! An example, not a binary of the package: it is built only on request and never shipped.
//!
//! Usage: `fake-daemon <dir>`
#[path = "../tests/support/mod.rs"]
mod support;

use support::{FakeDaemon, Harness, info};
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: fake-daemon <dir>"))?;
    let dir = std::path::PathBuf::from(dir);
    let daemon = FakeDaemon::start_in(&dir, Harness::new(vec![]), info(vec![])).await;
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    tokio::select! { _ = terminate.recv() => {}, _ = interrupt.recv() => {} }
    daemon.stop().await;
    Ok(())
}
