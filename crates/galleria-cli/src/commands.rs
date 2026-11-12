use crate::{cli::*, messages::*, prompt::*};
use galleria_ipc::{
    client::{Client, ClientError},
    *,
};
use galleria_profile::{FileSecretStore, ProfileStore, SecretStore};
use std::io::Write;
use zeroize::Zeroizing;
#[derive(Debug)]
pub struct CliError {
    pub code: u8,
    pub message: String,
}
pub fn exit_code(code: &str) -> u8 {
    match code {
        "ok" => 0,
        "auth_failed" => 2,
        "needs_input" => 3,
        "server_unreachable" => 4,
        "permission_denied" | "daemon_unreachable" | "incompatible_protocol" => 5,
        _ => 1,
    }
}
pub fn failure(code: &str) -> CliError {
    CliError {
        code: exit_code(code),
        message: text(code, Lang::from_env()).into(),
    }
}
fn storage(e: impl std::fmt::Display) -> CliError {
    CliError {
        code: 1,
        message: e.to_string(),
    }
}
fn ipc(e: ClientError) -> CliError {
    match e {
        ClientError::PermissionDenied => failure("permission_denied"),
        ClientError::Rpc(e) => {
            let data = e.data.unwrap_or_default();
            let code = data["code"].as_str().unwrap_or("internal");
            if code == "incompatible_protocol" {
                CliError {
                    code: 5,
                    message: incompatible_message(&data, Lang::from_env()),
                }
            } else {
                failure(code)
            }
        }
        _ => failure("daemon_unreachable"),
    }
}
pub fn status_json(status: &Status) -> Result<String, CliError> {
    serde_json::to_string(status).map_err(|_| failure("internal"))
}
async fn daemon(socket: &std::path::Path) -> Result<Client, CliError> {
    let mut c = Client::connect(socket).await.map_err(ipc)?;
    c.hello("galleria", env!("CARGO_PKG_VERSION"))
        .await
        .map_err(ipc)?;
    Ok(c)
}
fn print_log(entry: &LogEntry) -> Result<(), CliError> {
    println!(
        "{} {:?} {:?}: {}",
        entry.ts, entry.level, entry.source, entry.message
    );
    std::io::stdout().flush().map_err(|_| failure("internal"))
}
pub async fn run(cli: Cli) -> Result<(), CliError> {
    let dir = ProfileStore::default_dir();
    match cli.command {
        Command::Import { file, name } => {
            let text = Zeroizing::new(std::fs::read_to_string(&file).map_err(storage)?);
            let name = name.unwrap_or_else(|| {
                file.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let imported = match file.extension().and_then(|s| s.to_str()) {
                Some("conf") => galleria_profile::import_wireguard(&text, &name),
                Some("ovpn") => galleria_profile::import_openvpn_bytes(&text, &name, &|path| {
                    std::fs::read(
                        file.parent()
                            .unwrap_or(std::path::Path::new("."))
                            .join(path),
                    )
                }),
                _ => return Err(storage("Use a .conf or .ovpn profile")),
            }
            .map_err(storage)?;
            let profiles = ProfileStore::open(&dir).map_err(storage)?;
            let secrets = FileSecretStore::open(&dir).map_err(storage)?;
            for (key, value) in &imported.secrets {
                secrets
                    .set(&imported.profile.id, key, value)
                    .map_err(storage)?;
            }
            if let Err(e) = profiles.save(&imported.profile) {
                let _ = secrets.delete_all(&imported.profile.id);
                return Err(storage(e));
            }
            for warning in imported.warnings {
                eprintln!(
                    "line {}: {} ({:?})",
                    warning.line, warning.directive, warning.kind
                );
            }
            println!("{} {}", imported.profile.id, imported.profile.name);
        }
        Command::List => {
            for p in ProfileStore::open(&dir)
                .map_err(storage)?
                .list()
                .map_err(storage)?
            {
                println!("{} {} {:?}", p.id, p.name, p.protocol);
            }
        }
        Command::Delete { profile } => {
            let store = ProfileStore::open(&dir).map_err(storage)?;
            let p = store.find(&profile).map_err(storage)?;
            store.delete(&p.id).map_err(storage)?;
        }
        Command::Connect {
            profile,
            replace,
            mut username,
            password_stdin,
            otp_stdin,
            no_wait,
        } => {
            let store = ProfileStore::open(&dir).map_err(storage)?;
            let p = store.find(&profile).map_err(storage)?;
            let secrets = FileSecretStore::open(&dir).map_err(storage)?;
            let mut params =
                galleria_profile::to_connect_params(&p, &secrets, replace).map_err(storage)?;
            let values = stdin_secrets(&mut std::io::stdin().lock(), password_stdin, otp_stdin)?;
            if let BackendParams::Openvpn(p) = &mut params.backend {
                p.username = username.clone();
                p.password = values.password;
                p.otp = values.otp;
            }
            let mut c = daemon(&cli.socket).await?;
            let _: serde_json::Value = c
                .call(
                    "subscribe",
                    &SubscribeParams {
                        stats: false,
                        logs: false,
                    },
                )
                .await
                .map_err(ipc)?;
            let accepted: ConnectResult = c.call("connect", &params).await.map_err(ipc)?;
            drop(params);
            if no_wait {
                return Ok(());
            }
            let mut answered = None;
            let mut status: Status = c
                .call("status", &serde_json::json!({}))
                .await
                .map_err(ipc)?;
            loop {
                if status.session == Some(accepted.session) {
                    match status.state {
                        State::Connected => {
                            println!("connected");
                            return Ok(());
                        }
                        State::Error => {
                            let code = serde_json::to_value(status.error.map(|e| e.code))
                                .map_err(|_| failure("internal"))?;
                            return Err(failure(code.as_str().unwrap_or("backend_exited")));
                        }
                        State::NeedsInput => {
                            let request = status
                                .input_request
                                .as_ref()
                                .ok_or_else(|| failure("internal"))?;
                            let identity = (status.attempt, request.request, request.kind);
                            // A status call can queue the very event it reports.
                            if answered != Some(identity) {
                                let values = request_input(request, &mut username)?;
                                let _: serde_json::Value = c
                                    .call(
                                        "provide_input",
                                        &ProvideInputParams {
                                            session: accepted.session,
                                            request: request.request,
                                            values,
                                        },
                                    )
                                    .await
                                    .map_err(ipc)?;
                                answered = Some(identity);
                            }
                        }
                        State::Disconnected => return Err(failure("backend_exited")),
                        _ => {}
                    }
                } else if matches!(status.state, State::Disconnected | State::Error) {
                    return Err(failure("backend_exited"));
                }
                loop {
                    match c.next_event().await {
                        Some(Event::State(s)) => {
                            status = s;
                            break;
                        }
                        Some(_) => {}
                        None => return Err(failure("daemon_unreachable")),
                    }
                }
            }
        }
        Command::Disconnect => {
            let mut c = daemon(&cli.socket).await?;
            let _: serde_json::Value = c
                .call("disconnect", &serde_json::json!({}))
                .await
                .map_err(ipc)?;
            println!("disconnected");
        }
        Command::Status { json } => {
            let mut c = daemon(&cli.socket).await?;
            let status: Status = c
                .call("status", &serde_json::json!({}))
                .await
                .map_err(ipc)?;
            if json {
                println!("{}", status_json(&status)?);
            } else {
                println!(
                    "{}",
                    serde_json::to_value(status.state)
                        .map_err(|_| failure("internal"))?
                        .as_str()
                        .unwrap_or("unknown")
                );
            }
        }
        Command::Logs { follow } => {
            let mut c = daemon(&cli.socket).await?;
            if follow {
                let _: serde_json::Value = c
                    .call(
                        "subscribe",
                        &SubscribeParams {
                            stats: false,
                            logs: true,
                        },
                    )
                    .await
                    .map_err(ipc)?;
            }
            let logs: LogsResult = c
                .call(
                    "logs",
                    &LogsParams {
                        after: None,
                        limit: 1000,
                    },
                )
                .await
                .map_err(ipc)?;
            let mut seq = logs.last;
            for entry in logs.entries {
                print_log(&entry)?;
            }
            if follow {
                loop {
                    match c.next_event().await {
                        Some(Event::Log(l)) if l.seq > seq => {
                            seq = l.seq;
                            print_log(&l)?;
                        }
                        Some(_) => {}
                        None => return Err(failure("daemon_unreachable")),
                    }
                }
            }
        }
        Command::Version => {
            let mut c = daemon(&cli.socket).await?;
            let hello = c
                .hello("galleria", env!("CARGO_PKG_VERSION"))
                .await
                .map_err(ipc)?;
            println!(
                "galleria {}\ngalleriad {}\n{}",
                env!("CARGO_PKG_VERSION"),
                hello.daemon_version,
                serde_json::to_string(&hello.backends).map_err(|_| failure("internal"))?
            );
        }
    }
    Ok(())
}
