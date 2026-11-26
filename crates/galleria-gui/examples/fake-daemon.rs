//! A daemon for the WebDriver smoke test (lab/tests/gui-smoke.sh): the real `galleriad::server::serve` with a fake
//! backend, listening on `<dir>/galleriad.sock` until SIGTERM or SIGINT. It exists because the real `galleriad run`
//! cleans the network state when it starts (`ip rule del …`), which a smoke test must never do (S5, spec §17).
//! An example, not a binary of the package: it is built only on request and never shipped.
//!
//! Usage: `fake-daemon <dir> [--scenario <name>]`. A scenario is a list of scripts of the fake backend, one for each
//! `connect` (plan B5, P16; the names are in `tests/support/scenarios.rs`). Without one the backend says nothing.
#[path = "../tests/support/mod.rs"]
mod support;

use support::{FakeDaemon, Harness, info, scenarios};
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let dir = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: fake-daemon <dir> [--scenario <name>]"))?;
    let dir = std::path::PathBuf::from(dir);
    let scripts = match (args.next(), args.next()) {
        (None, _) => vec![],
        (Some(flag), Some(name)) if flag == "--scenario" => {
            let name = name.to_string_lossy().into_owned();
            scenarios::scenario(&name).ok_or_else(|| {
                anyhow::anyhow!("no such scenario: {name} (one of {:?})", scenarios::NAMES)
            })?
        }
        _ => anyhow::bail!("usage: fake-daemon <dir> [--scenario <name>]"),
    };
    let daemon =
        FakeDaemon::start_in(&dir, Harness::new(scripts), info(scenarios::backends())).await;
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    tokio::select! { _ = terminate.recv() => {}, _ = interrupt.recv() => {} }
    daemon.stop().await;
    Ok(())
}
