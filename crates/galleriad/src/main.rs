#![deny(unsafe_code)]
use clap::{Parser, Subcommand};
use galleria_backend::net::NetConfig;
use galleria_ipc::{ErrorCode, SessionError};
use galleriad::{
    config::DaemonConfig,
    machine::RetryPolicy,
    manager::{BackendFactory, NetControl, spawn_manager},
    server::{DaemonInfo, serve},
};
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
#[derive(Parser)]
#[command(version, about = "Galleria system daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    Run {
        #[arg(long)]
        socket: Option<PathBuf>,
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        runtime_dir: Option<PathBuf>,
    },
}
struct NoNet;
impl NetControl for NoNet {
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
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut cfg = DaemonConfig::default();
    if let Some(Command::Run {
        socket,
        group,
        runtime_dir,
    }) = cli.command
    {
        if let Some(p) = socket {
            cfg.socket = p;
        }
        if let Some(g) = group {
            cfg.group = g;
        }
        if let Some(p) = runtime_dir {
            cfg.runtime_dir = p;
        }
    }
    galleriad::harden_process();
    let factory: BackendFactory =
        Arc::new(|_, _| Err(ErrorCode::BackendUnavailable.to_rpc("No backend available")));
    let manager = spawn_manager(
        factory,
        Arc::new(NoNet),
        RetryPolicy::default(),
        cfg.runtime_dir.clone(),
    );
    let shutdown = CancellationToken::new();
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let token = shutdown.clone();
    let signals = tokio::spawn(async move {
        tokio::select! {_=terminate.recv()=>{},_=interrupt.recv()=>{}}
        token.cancel();
    });
    let result = serve(
        cfg,
        manager,
        DaemonInfo {
            version: env!("CARGO_PKG_VERSION").into(),
            backends: vec![],
        },
        shutdown,
    )
    .await;
    signals.abort();
    result
}
