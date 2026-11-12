#![forbid(unsafe_code)]
use clap::Parser;
#[tokio::main]
async fn main() {
    let cli = match galleria_cli::cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let help = matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            );
            let _ = error.print();
            std::process::exit(if help { 0 } else { 1 });
        }
    };
    if let Err(error) = galleria_cli::commands::run(cli).await {
        eprintln!("{}", error.message);
        std::process::exit(i32::from(error.code));
    }
}
