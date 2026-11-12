use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
#[command(
    name = "galleria",
    about = "Control the Galleria VPN daemon",
    disable_version_flag = true
)]
pub struct Cli {
    #[arg(long,global=true,default_value=galleria_ipc::DEFAULT_SOCKET_PATH)]
    pub socket: PathBuf,
    #[command(subcommand)]
    pub command: Command,
}
#[derive(Subcommand)]
pub enum Command {
    Import {
        file: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    List,
    Delete {
        profile: String,
    },
    Connect {
        profile: String,
        #[arg(long)]
        replace: bool,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long)]
        otp_stdin: bool,
        #[arg(long)]
        no_wait: bool,
    },
    Disconnect,
    Status {
        #[arg(long)]
        json: bool,
    },
    Logs {
        #[arg(short = 'f', long)]
        follow: bool,
    },
    Version,
}
