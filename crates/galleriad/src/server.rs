use crate::{
    auth::*, config::DaemonConfig, dispatch::dispatch, manager::ManagerHandle,
    session_dir::purge_session_dirs,
};
use galleria_ipc::*;
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{Semaphore, broadcast, mpsc},
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct DaemonInfo {
    pub version: String,
    pub backends: Vec<BackendInfo>,
    pub backend_probe: Option<std::sync::Arc<dyn Fn() -> Vec<BackendInfo> + Send + Sync>>,
}
struct SocketGuard {
    path: PathBuf,
    dev: u64,
    ino: u64,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        if fs::symlink_metadata(&self.path)
            .is_ok_and(|m| m.dev() == self.dev && m.ino() == self.ino)
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}
fn startup_lock(socket: &Path) -> anyhow::Result<File> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(socket.with_extension("sock.lock"))?;
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| anyhow::anyhow!("Daemon already running"))?;
    Ok(lock)
}
async fn remove_stale_socket(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            anyhow::ensure!(meta.file_type().is_socket(), "Socket path is not a socket");
            match UnixStream::connect(path).await {
                Ok(_) => anyhow::bail!("Daemon already running"),
                Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => fs::remove_file(path)?,
                Err(e) => return Err(e.into()),
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
pub async fn serve(
    cfg: DaemonConfig,
    manager: ManagerHandle,
    info: DaemonInfo,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    fs::create_dir_all(&cfg.runtime_dir)?;
    if let Some(parent) = cfg.socket.parent() {
        fs::create_dir_all(parent)?;
    }
    let _lock = startup_lock(&cfg.socket)?;
    remove_stale_socket(&cfg.socket).await?;
    manager.recover().await?;
    purge_session_dirs(&cfg.runtime_dir)?;
    let allowed_gid = nix::unistd::Group::from_name(&cfg.group)?.map(|g| g.gid.as_raw());
    let listener = UnixListener::bind(&cfg.socket)?;
    let meta = fs::symlink_metadata(&cfg.socket)?;
    let _guard = SocketGuard {
        path: cfg.socket.clone(),
        dev: meta.dev(),
        ino: meta.ino(),
    };
    // Restrict before assigning group, then expose only after authorization is configured.
    fs::set_permissions(&cfg.socket, fs::Permissions::from_mode(0o600))?;
    if let Some(gid) = allowed_gid {
        std::os::unix::fs::chown(&cfg.socket, None, Some(gid))?;
        fs::set_permissions(&cfg.socket, fs::Permissions::from_mode(0o660))?;
    } else {
        eprintln!("warning: configured group not found; socket restricted to daemon owner");
    }
    let policy = AccessPolicy { allowed_gid };
    let semaphore = Arc::new(Semaphore::new(cfg.max_clients));
    let info = Arc::new(info);
    let mut clients = JoinSet::new();
    let result = loop {
        while clients.try_join_next().is_some() {}
        tokio::select! {
         biased;
         _=shutdown.cancelled()=>break Ok(()),
         accepted=listener.accept()=>{
          let (stream,_)=match accepted{Ok(s)=>s,Err(e)=>break Err(e.into())};
          let creds=match peer_creds(&stream){Ok(c)=>c,Err(_)=>continue};
          if !authorize(&creds,&policy){continue;}
          let permit=semaphore.clone().try_acquire_owned();
          let manager=manager.clone();let info=info.clone();let hello_timeout=cfg.hello_timeout;
          admit(stream,permit,&mut clients,move|stream|async move {
           let _=connection(stream,creds.uid,manager,info,hello_timeout).await;
          });
         },
         _=clients.join_next(),if !clients.is_empty()=>{},
        }
    };
    manager.disconnect(0).await;
    clients.abort_all();
    while clients.join_next().await.is_some() {}
    drop(listener);
    result
}
struct AbortReader(tokio::task::JoinHandle<()>);
impl Drop for AbortReader {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn connection(
    stream: UnixStream,
    uid: u32,
    manager: ManagerHandle,
    info: Arc<DaemonInfo>,
    hello_timeout: Duration,
) -> io::Result<()> {
    let (read, mut write) = stream.into_split();
    let (tx, mut frames) = mpsc::channel(8);
    // Keep read_frame alive across event writes: it owns a partial frame and is not cancellation safe.
    let reader = AbortReader(tokio::spawn(async move {
        let mut read = BufReader::with_capacity(8192, read);
        loop {
            let frame = read_frame(&mut read, MAX_MESSAGE_BYTES).await;
            let done = !matches!(&frame, Ok(Some(_)));
            if tx.send(frame).await.is_err() || done {
                break;
            }
        }
    }));
    let deadline = tokio::time::Instant::now() + hello_timeout;
    let mut hello = false;
    let mut subscriptions = SubscribeParams::default();
    let mut events = manager.events();
    loop {
        let write_deadline = (!hello).then_some(deadline);
        tokio::select! {
         _=tokio::time::sleep_until(deadline),if !hello=>break,
         frame=frames.recv()=>{
          let bytes=match frame{
           Some(Ok(Some(bytes)))=>bytes,
           Some(Err(FrameError::TooLarge))=>{send_frame(&mut write, response_err(None,&ErrorCode::MessageTooLarge.to_rpc("Message too large")), write_deadline).await?;break;},
           _=>break,
          };
          let request=match decode_line(&bytes){Ok(r)=>r,Err(e)=>{send_frame(&mut write, response_err(None,&e), write_deadline).await?;continue;}};
          if !hello&&request.method!="hello"{send_frame(&mut write, response_err(Some(&request.id),&ErrorCode::HelloRequired.to_rpc("Hello required")), write_deadline).await?;continue;}
          let call=match parse_call(&request.method,request.params){Ok(c)=>c,Err(mut e)=>{
           let incompatible=e.data.as_ref().is_some_and(|d|d["code"]=="incompatible_protocol");
           if incompatible&&let Some(data)=e.data.as_mut(){data["daemon_version"]=serde_json::json!(info.version);}
           send_frame(&mut write, response_err(Some(&request.id),&e), write_deadline).await?;if incompatible{break;}continue;
          }};
          if let Call::Subscribe(params) = &call { subscriptions = params.clone(); }
          let is_hello=matches!(call,Call::Hello(_));let response=dispatch(call,&manager,&info,uid).await;
          match response{Ok(value)=>{send_frame(&mut write, response_ok(&request.id,value), write_deadline).await?;if is_hello{hello=true;}},Err(e)=>send_frame(&mut write, response_err(Some(&request.id),&e), write_deadline).await?,}
         },
         event=events.recv(),if hello=>match event{
          Ok(event @ Event::State(_))=>write.write_all(&notification(&event)).await?,
          Ok(event @ Event::Stats(_)) if subscriptions.stats => write.write_all(&notification(&event)).await?,
          Ok(event @ Event::Log(_)) if subscriptions.logs => write.write_all(&notification(&event)).await?,
          Ok(Event::Stats(_)|Event::Log(_))=>{},
          Err(broadcast::error::RecvError::Lagged(_))=>{write.write_all(&notification(&Event::State(manager.status().await))).await?;},
          Err(broadcast::error::RecvError::Closed)=>break,
         },
        }
    }
    drop(reader);
    write.shutdown().await
}

async fn send_frame(
    write: &mut tokio::net::unix::OwnedWriteHalf,
    frame: Vec<u8>,
    deadline: Option<tokio::time::Instant>,
) -> io::Result<()> {
    match deadline {
        Some(deadline) => tokio::time::timeout_at(deadline, write.write_all(&frame))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Hello deadline expired"))?,
        None => write.write_all(&frame).await,
    }
}

fn admit<F: std::future::Future<Output = ()> + Send + 'static>(
    stream: UnixStream,
    permit: Result<tokio::sync::OwnedSemaphorePermit, tokio::sync::TryAcquireError>,
    clients: &mut JoinSet<()>,
    run: impl FnOnce(UnixStream) -> F + Send + 'static,
) {
    match permit {
        Ok(permit) => {
            clients.spawn(async move {
                let _permit = permit;
                run(stream).await;
            });
        }
        Err(_) => {
            // into_std preserves nonblocking mode; admission errors never allocate handler tasks
            // or wait for a peer to read. A new socket has room for this short response.
            if let Ok(mut stream) = stream.into_std() {
                let _ = std::io::Write::write_all(
                    &mut stream,
                    &response_err(None, &ErrorCode::TooManyClients.to_rpc("Too many clients")),
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn rejected_connections_do_not_allocate_unbounded_handler_tasks() {
        let semaphore = Arc::new(Semaphore::new(0));
        let mut clients = JoinSet::new();
        for _ in 0..128 {
            let (stream, peer) = UnixStream::pair().unwrap();
            admit(
                stream,
                semaphore.clone().try_acquire_owned(),
                &mut clients,
                |_| async { panic!("rejected handler started") },
            );
            assert!(clients.is_empty(), "rejections must not accumulate tasks");
            let mut reader = BufReader::new(peer);
            let value: serde_json::Value = serde_json::from_slice(
                &read_frame(&mut reader, MAX_MESSAGE_BYTES)
                    .await
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(value["error"]["data"]["code"], "too_many_clients");
        }
    }
}
