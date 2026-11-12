use crate::{
    config::*,
    hookenv::ProfileFacts,
    session::{Credentials, OpenVpnBackend, drive_with_output},
    version::*,
};
use galleria_backend::*;
use galleria_ipc::*;
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    sync::mpsc,
};
use zeroize::Zeroizing;
#[derive(Debug, Clone)]
pub struct OpenVpnBinary {
    pub path: PathBuf,
    pub version: OpenVpnVersion,
}
#[derive(Debug)]
pub enum Unavailable {
    NotFound,
    TooOld(OpenVpnVersion),
}
pub fn probe() -> Result<OpenVpnBinary, Unavailable> {
    for path in ["/usr/sbin/openvpn", "/usr/bin/openvpn"] {
        let Ok(out) = std::process::Command::new(path)
            .arg("--version")
            .env("LC_ALL", "C")
            .output()
        else {
            continue;
        };
        let Some(version) = String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .and_then(parse_version)
        else {
            continue;
        };
        if version < MIN_VERSION {
            return Err(Unavailable::TooOld(version));
        }
        return Ok(OpenVpnBinary {
            path: path.into(),
            version,
        });
    }
    Err(Unavailable::NotFound)
}
struct Abort(tokio::task::JoinHandle<()>);
impl Drop for Abort {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn logs<R: AsyncRead + Unpin>(input: R, events: mpsc::Sender<BackendEvent>) {
    let mut lines = BufReader::new(input).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if events
            .send(BackendEvent::Log(LogLine {
                level: LogLevel::Info,
                message: line,
            }))
            .await
            .is_err()
        {
            break;
        }
    }
}
pub(crate) async fn run(
    mut backend: OpenVpnBackend,
    ctx: &SessionContext,
    mut commands: mpsc::Receiver<BackendCommand>,
) -> ExitReason {
    let error = || ExitReason::Transport("Cannot start OpenVPN".into());
    let mgmt_socket = ctx.session_dir.join("mgmt.sock");
    let hook_socket = ctx.session_dir.join("hook.sock");
    let Ok(listener) = tokio::net::UnixListener::bind(&hook_socket) else {
        return error();
    };
    let (calls, hooks) = mpsc::channel(8);
    let _helper = Abort(tokio::spawn(crate::hook::serve_hook(listener, calls)));
    let config = Zeroizing::new(generate_config(&GenInput {
        profile: &backend.cfg,
        version: backend.bin.version,
        mgmt_socket: &mgmt_socket,
        hook_socket: &hook_socket,
        daemon_exe: &backend.daemon_exe,
        options: &backend.options,
    }));
    let child = tokio::process::Command::new(&backend.bin.path)
        .args(["--config", "/dev/stdin"])
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let Ok(mut child) = child else {
        return error();
    };
    let stdout = child.stdout.take();
    let _stderr = child
        .stderr
        .take()
        .map(|s| Abort(tokio::spawn(logs(s, ctx.events.clone()))));
    let written = if let Some(mut stdin) = child.stdin.take() {
        matches!(
            tokio::time::timeout(Duration::from_secs(5), stdin.write_all(config.as_bytes())).await,
            Ok(Ok(()))
        )
    } else {
        false
    };
    drop(config);
    if !written {
        let _ = child.kill().await;
        return error();
    }
    let connect = async {
        loop {
            if let Ok(stream) = tokio::net::UnixStream::connect(&mgmt_socket).await {
                return Some(stream);
            }
            if child.try_wait().ok().flatten().is_some() {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    };
    let stream = tokio::select! {
     result=tokio::time::timeout(Duration::from_secs(10),connect)=>result.ok().flatten(),
     _=commands.recv()=>{let _=child.kill().await;return ExitReason::Stopped;}
    };
    let Some(stream) = stream else {
        let _ = child.kill().await;
        return error();
    };
    let creds = Credentials::new(
        std::mem::take(&mut backend.cfg.credentials),
        ProfileFacts {
            redirect: profile_redirect(&backend.cfg),
            ipv6_routes: profile_ipv6_routes(&backend.cfg),
        },
        backend.options.ipv6_block,
    )
    .ignore_pushed_routes(backend.options.ignore_pushed_routes)
    .pull_filters(profile_pull_filters(&backend.cfg));
    let Some(stdout) = stdout else {
        let _ = child.kill().await;
        return error();
    };
    let reason = drive_with_output(stream, creds, ctx, commands, hooks, stdout).await;
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill().await;
    } else {
        let _ = child.wait().await;
    }
    reason
}
