use crate::{config::*, hook::*, hookenv::*, mgmt::*, push::*, queue::CommandQueue};
use galleria_backend::*;
use galleria_ipc::*;
use std::time::{Duration, Instant};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    sync::{mpsc, oneshot},
};
use zeroize::Zeroizing;
pub struct Credentials {
    values: InputValues,
    profile: ProfileFacts,
    ipv6_block: bool,
    ignore_routes: bool,
}
impl Credentials {
    pub fn new(values: InputValues, profile: ProfileFacts, ipv6_block: bool) -> Self {
        Self {
            values,
            profile,
            ipv6_block,
            ignore_routes: false,
        }
    }
    pub fn ignore_pushed_routes(mut self, ignore: bool) -> Self {
        self.ignore_routes = ignore;
        self
    }
}
pub struct OpenVpnBackend {
    pub(crate) cfg: ValidOpenVpn,
    pub(crate) bin: crate::plat::OpenVpnBinary,
    pub(crate) options: ConnectOptions,
    pub(crate) daemon_exe: std::path::PathBuf,
}
impl OpenVpnBackend {
    pub fn new(
        cfg: ValidOpenVpn,
        bin: crate::plat::OpenVpnBinary,
        options: ConnectOptions,
        daemon_exe: std::path::PathBuf,
    ) -> Self {
        Self {
            cfg,
            bin,
            options,
            daemon_exe,
        }
    }
}
pub fn capabilities() -> Capabilities {
    Capabilities {
        handshake: false,
        interactive_auth: true,
        server_pushed_config: true,
    }
}
impl Backend for OpenVpnBackend {
    fn protocol(&self) -> Protocol {
        Protocol::Openvpn
    }
    fn capabilities(&self) -> Capabilities {
        capabilities()
    }
    fn spawn(self: Box<Self>, ctx: SessionContext) -> SessionHandle {
        let (commands, rx) = mpsc::channel(16);
        let task = tokio::spawn(async move {
            let reason = crate::plat::run(*self, &ctx, rx).await;
            let _ = ctx.events.send(BackendEvent::Exited(reason)).await;
        });
        SessionHandle { commands, task }
    }
}
enum Pending {
    Up {
        receive: oneshot::Receiver<Result<(), String>>,
        verdict: oneshot::Sender<bool>,
    },
    Down {
        receive: oneshot::Receiver<()>,
        verdict: oneshot::Sender<bool>,
    },
}
async fn pending_reply(p: &mut Option<Pending>) -> bool {
    match p {
        Some(Pending::Up { receive, .. }) => matches!(receive.await, Ok(Ok(()))),
        Some(Pending::Down { receive, .. }) => {
            let _ = receive.await;
            true
        }
        None => std::future::pending().await,
    }
}
async fn frame<R: AsyncBufRead + Unpin>(
    read: &mut R,
    partial: &mut Vec<u8>,
) -> Result<Option<String>, ()> {
    loop {
        let bytes = read.fill_buf().await.map_err(|_| ())?;
        if bytes.is_empty() {
            return if partial.is_empty() {
                Ok(None)
            } else {
                Err(())
            };
        }
        let end = bytes.iter().position(|b| *b == b'\n');
        let n = end.map_or(bytes.len(), |n| n + 1);
        if partial.len() + n > galleria_ipc::MAX_MESSAGE_BYTES {
            return Err(());
        }
        partial.extend_from_slice(&bytes[..n]);
        read.consume(n);
        if end.is_some() {
            let text = String::from_utf8(std::mem::take(partial)).map_err(|_| ())?;
            return Ok(Some(text));
        }
    }
}
fn failure() -> ExitReason {
    ExitReason::Transport("OpenVPN management connection closed".into())
}
fn fatal(code: SessionErrorCode) -> ExitReason {
    ExitReason::Fatal(SessionError::new(code))
}
#[derive(Clone)]
enum Prompt {
    Auth(Option<StaticChallenge>),
    Key,
}
async fn answer(
    prompt: &Prompt,
    values: &mut InputValues,
    q: &mut CommandQueue,
    ctx: &SessionContext,
) -> bool {
    let request = match prompt {
        Prompt::Key => {
            if let Some(p) = &values.key_passphrase {
                q.push(format!(
                    "password \"Private Key\" {}",
                    mgmt_quote(p.expose())
                ));
                return true;
            }
            InputRequest {
                request: 0,
                kind: InputKind::KeyPassphrase,
                prompt: "Private key passphrase".into(),
                echo: false,
                expires_in_secs: 120,
            }
        }
        Prompt::Auth(sc) => {
            if let (Some(user), Some(password)) = (&values.username, &values.password) {
                if sc.is_none() || values.otp.is_some() {
                    let response = Zeroizing::new(auth_response(
                        password.expose(),
                        values.otp.as_ref().map(Secret::expose),
                        sc.as_ref(),
                    ));
                    q.push(format!("username \"Auth\" {}", mgmt_quote(user)));
                    q.push(format!("password \"Auth\" {}", mgmt_quote(&response)));
                    if sc.is_some() {
                        values.otp = None;
                    }
                    return true;
                }
                let Some(sc) = sc else {
                    return false;
                };
                InputRequest {
                    request: 0,
                    kind: InputKind::Otp,
                    prompt: sc.text.clone(),
                    echo: sc.echo,
                    expires_in_secs: 120,
                }
            } else {
                InputRequest {
                    request: 0,
                    kind: InputKind::Credentials,
                    prompt: "Username and password".into(),
                    echo: false,
                    expires_in_secs: 120,
                }
            }
        }
    };
    let _ = ctx.events.send(BackendEvent::InputRequired(request)).await;
    false
}
fn merge(values: &mut InputValues, new: InputValues) {
    if new.username.is_some() {
        values.username = new.username;
    }
    if new.password.is_some() {
        values.password = new.password;
    }
    if new.otp.is_some() {
        values.otp = new.otp;
    }
    if new.key_passphrase.is_some() {
        values.key_passphrase = new.key_passphrase;
    }
}
pub async fn drive<S: AsyncRead + AsyncWrite + Unpin>(
    mgmt: S,
    creds: Credentials,
    ctx: &SessionContext,
    commands: mpsc::Receiver<BackendCommand>,
    hook: mpsc::Receiver<HookCall>,
) -> ExitReason {
    drive_with_output(mgmt, creds, ctx, commands, hook, tokio::io::empty()).await
}
pub async fn drive_with_output<S: AsyncRead + AsyncWrite + Unpin, R: AsyncRead + Unpin>(
    mgmt: S,
    mut creds: Credentials,
    ctx: &SessionContext,
    mut commands: mpsc::Receiver<BackendCommand>,
    mut hook: mpsc::Receiver<HookCall>,
    output: R,
) -> ExitReason {
    let (read, mut write) = tokio::io::split(mgmt);
    let mut read = BufReader::new(read);
    let mut partial = Vec::new();
    let mut output = BufReader::new(output);
    let mut output_partial = Vec::new();
    let mut output_open = true;
    let mut q = CommandQueue::default();
    let mut setup = false;
    let mut hold = false;
    let mut hold_queued = false;
    let mut accepted = false;
    let mut was_up = false;
    let mut restarting = false;
    let mut pushed = None;
    let mut management_push = PushAccumulator::default();
    let mut output_push = PushAccumulator::default();
    let mut pending = None;
    let mut prompt = None;
    let mut stopping = None;
    let mut last_stats = None;
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    loop {
        if let Some(cmd) = q.next_to_send(Instant::now()) {
            let cmd = Zeroizing::new(format!("{cmd}\n"));
            if write.write_all(cmd.as_bytes()).await.is_err() {
                return if stopping.is_some() {
                    ExitReason::Stopped
                } else {
                    failure()
                };
            }
        }
        tokio::select! {
            biased;
            command = commands.recv(), if stopping.is_none() => match command {
                Some(BackendCommand::Stop) | None => {
                    stopping = Some(Instant::now());
                    q = CommandQueue::default();
                    q.push("signal SIGTERM".into());
                }
                Some(BackendCommand::ProvideInput(input)) => {
                    merge(&mut creds.values, input);
                    if let Some(p) = &prompt
                        && answer(p, &mut creds.values, &mut q, ctx).await
                    {
                        prompt = None;
                    }
                }
            },
            ok = pending_reply(&mut pending), if pending.is_some() => {
                match pending.take() {
                    Some(Pending::Up { verdict, .. }) => {
                        accepted = ok;
                        let _ = verdict.send(ok);
                        if !ok && stopping.is_none() {
                            return fatal(SessionErrorCode::NetSetupFailed);
                        }
                    }
                    Some(Pending::Down { verdict, .. }) => { let _ = verdict.send(true); }
                    None => {}
                }
            },
            line = frame(&mut read, &mut partial) => {
                let line = match line {
                    Ok(Some(s)) => s,
                    _ => return if stopping.is_some() { ExitReason::Stopped } else { failure() },
                };
                match parse_line(&line) {
                    MgmtEvent::Info(_) => {
                        if !setup {
                            setup = true;
                            for cmd in ["version 3", "state on", "bytecount 1", "log on"] {
                                q.push(cmd.into());
                            }
                            if hold && !hold_queued {
                                q.push("hold release".into());
                                hold_queued = true;
                            }
                        }
                    }
                    MgmtEvent::Hold => {
                        hold = true;
                        if setup && !hold_queued {
                            q.push("hold release".into());
                            hold_queued = true;
                        }
                    }
                    MgmtEvent::Success(_) | MgmtEvent::Error(_) => q.on_reply(),
                    MgmtEvent::NeedAuth { static_challenge } => {
                        let p = Prompt::Auth(static_challenge);
                        if !answer(&p, &mut creds.values, &mut q, ctx).await { prompt = Some(p); }
                    }
                    MgmtEvent::NeedKeyPassphrase => {
                        if !answer(&Prompt::Key, &mut creds.values, &mut q, ctx).await {
                            prompt = Some(Prompt::Key);
                        }
                    }
                    MgmtEvent::NeedDynamicChallenge => return fatal(SessionErrorCode::UnsupportedChallenge),
                    MgmtEvent::VerificationFailed(_) => return ExitReason::AuthFailed("Authentication failed".into()),
                    MgmtEvent::Fatal(_) => return if stopping.is_some() {
                        ExitReason::Stopped
                    } else {
                        ExitReason::Transport("OpenVPN reported a fatal error".into())
                    },
                    MgmtEvent::State { name, detail, local_ip, remote } => match name.as_str() {
                        "CONNECTED" => {
                            if accepted && stopping.is_none() {
                                was_up = true;
                                restarting = false;
                                let _ = ctx.events.send(BackendEvent::Up(TunnelInfo {
                                    local_addresses: local_ip.into_iter().collect(), remote,
                                })).await;
                            }
                        }
                        "RECONNECTING" => {
                            accepted = false;
                            pushed = None;
                            management_push.clear();
                            output_push.clear();
                            if was_up && !restarting && stopping.is_none() {
                                restarting = true;
                                let _ = ctx.events.send(BackendEvent::Restarting(detail)).await;
                            }
                        }
                        "EXITING" => return if stopping.is_some() {
                            ExitReason::Stopped
                        } else if detail == "auth-failure" {
                            ExitReason::AuthFailed("Authentication failed".into())
                        } else { failure() },
                        _ => {
                            if stopping.is_none() {
                                let _ = ctx.events.send(BackendEvent::Progress(name)).await;
                            }
                        }
                    },
                    MgmtEvent::ByteCount { rx, tx } => {
                        if last_stats.is_none_or(|t: Instant| t.elapsed() >= Duration::from_secs(1)) {
                            last_stats = Some(Instant::now());
                            let _ = ctx.events.send(BackendEvent::Stats(Stats {
                                rx_bytes: rx, tx_bytes: tx, last_handshake: None,
                            })).await;
                        }
                    }
                    MgmtEvent::Log { flags, text } => {
                        if let Some(p) = management_push.feed(&text, creds.ignore_routes) {
                            pushed = Some(p);
                        }
                        let level = if flags.contains(['F', 'E']) { LogLevel::Error }
                            else if flags.contains('W') { LogLevel::Warn } else { LogLevel::Info };
                        let _ = ctx.events.send(BackendEvent::Log(LogLine { level, message: text })).await;
                    }
                    MgmtEvent::Other(_) => {}
                }
            },
            // OpenVPN can delay management log delivery until after --up. The same PUSH_REPLY
            // is already in stdout before it executes the helper (spec S9 fallback).
            line = frame(&mut output, &mut output_partial), if output_open => {
                match line {
                    Ok(Some(text)) => {
                        let text = text.trim_end_matches(['\r', '\n']);
                        if let Some(p) = output_push.feed(text, creds.ignore_routes) {
                            pushed = Some(p);
                        }
                        let _ = ctx.events.send(BackendEvent::Log(LogLine {
                            level: LogLevel::Info, message: text.into(),
                        })).await;
                    }
                    _ => output_open = false,
                }
            },
            call = hook.recv(), if pending.is_none() && !hook.is_closed() => {
                if let Some(call) = call {
                    match parse_hook_env(&call.env) {
                        Ok(env) if env.script_type == "up" => {
                            let (reply, receive) = oneshot::channel();
                            let config = net_config(&env, pushed.as_ref(), &creds.profile, creds.ipv6_block);
                            if ctx.events.send(BackendEvent::NetUp { config, reply }).await.is_err() {
                                return ExitReason::Stopped;
                            }
                            pending = Some(Pending::Up { receive, verdict: call.reply });
                        }
                        Ok(_) => {
                            accepted = false;
                            if was_up { pushed = None; management_push.clear(); output_push.clear(); }
                            let (reply, receive) = oneshot::channel();
                            if ctx.events.send(BackendEvent::NetDown { reply }).await.is_err() {
                                let _ = call.reply.send(true);
                                return ExitReason::Stopped;
                            }
                            pending = Some(Pending::Down { receive, verdict: call.reply });
                        }
                        Err(_) => { let _ = call.reply.send(false); }
                    }
                }
            },
            _ = tick.tick() => {
                if stopping.is_some_and(|t| t.elapsed() >= Duration::from_secs(10)) {
                    return ExitReason::Stopped;
                }
            }
        }
    }
}
