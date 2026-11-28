#![deny(unsafe_code)]
/// Windows: the daemon is not there yet (the service is P2); it says so and leaves with 2.
#[cfg(windows)]
fn main() {
    eprintln!("galleriad: Windows is not supported yet");
    std::process::exit(2);
}
#[cfg(unix)]
use clap::{Parser, Subcommand};
#[cfg(unix)]
use galleria_net::{NetManager, SystemRunner};
#[cfg(unix)]
use galleriad::{
    config::DaemonConfig,
    machine::RetryPolicy,
    manager::spawn_manager,
    server::{DaemonInfo, serve},
};
#[cfg(unix)]
use std::{path::PathBuf, sync::Arc};
#[cfg(unix)]
use tokio_util::sync::CancellationToken;
#[cfg(unix)]
#[derive(Parser)]
#[command(version, about = "Galleria system daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}
#[cfg(unix)]
#[derive(Subcommand)]
enum Command {
    OvpnHook {
        socket: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
    Run {
        #[arg(long)]
        socket: Option<PathBuf>,
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        runtime_dir: Option<PathBuf>,
    },
}
#[cfg(unix)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if let Some(Command::OvpnHook { socket, .. }) = &cli.command {
        std::process::exit(galleria_backend_openvpn::hook::hook_main(socket));
    }
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
    let factory = galleriad::factory::real_factory(std::env::current_exe()?);
    let net = NetManager::new(
        SystemRunner::locate()?,
        cfg.runtime_dir.join("net-journal.json"),
    );
    let manager = spawn_manager(
        factory,
        Arc::new(net),
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
            backend_probe: Some(Arc::new(galleriad::factory::backend_infos)),
        },
        shutdown,
    )
    .await;
    signals.abort();
    result
}
