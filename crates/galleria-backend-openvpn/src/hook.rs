use crate::hookenv::HOOK_ENV_PREFIXES;
use std::{collections::BTreeMap, path::Path};
use tokio::sync::{mpsc, oneshot};
pub struct HookCall {
    pub env: BTreeMap<String, String>,
    pub reply: oneshot::Sender<bool>,
}
pub fn filter_env(env: BTreeMap<String, String>) -> BTreeMap<String, String> {
    env.into_iter()
        .filter(|(k, _)| {
            HOOK_ENV_PREFIXES.iter().any(|p| {
                if p.ends_with('_') {
                    k.starts_with(p)
                } else {
                    k == p
                }
            })
        })
        .collect()
}
pub fn exit_code(kind: &str, verdict: Option<bool>) -> i32 {
    if kind == "down" || kind == "up" && verdict == Some(true) {
        0
    } else {
        1
    }
}
pub fn hook_main(socket: &Path) -> i32 {
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
        time::Duration,
    };
    let env = filter_env(
        std::env::vars_os()
            .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
            .collect(),
    );
    let kind = env.get("script_type").map_or("", String::as_str);
    let result = (|| -> Option<bool> {
        let mut s = UnixStream::connect(socket).ok()?;
        s.set_read_timeout(Some(Duration::from_secs(12))).ok()?;
        s.set_write_timeout(Some(Duration::from_secs(2))).ok()?;
        let mut bytes = serde_json::to_vec(&env).ok()?;
        if bytes.len() > 65535 {
            return None;
        }
        bytes.push(b'\n');
        s.write_all(&bytes).ok()?;
        let mut result = [0];
        s.read_exact(&mut result).ok()?;
        Some(result[0] == b'1')
    })();
    exit_code(kind, result)
}
pub(crate) async fn serve_hook(listener: tokio::net::UnixListener, calls: mpsc::Sender<HookCall>) {
    use tokio::io::{AsyncWriteExt, BufReader};
    let mut workers = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
         accepted=listener.accept()=>{let Ok((stream,_))=accepted else{break;};if !stream.peer_cred().is_ok_and(|c|c.uid()==0){continue;}let tx=calls.clone();
          if workers.len()>=8{continue;}
          workers.spawn(async move{let _=tokio::time::timeout(std::time::Duration::from_secs(12),async move{
           let mut reader=BufReader::new(stream);let Ok(Some(bytes))=galleria_ipc::read_frame(&mut reader,65536).await else{return;};
           let Ok(env)=serde_json::from_slice::<BTreeMap<String,String>>(&bytes) else{return;};let(reply,verdict)=oneshot::channel();
           if tx.send(HookCall{env:filter_env(env),reply}).await.is_ok(){let ok=verdict.await.unwrap_or(false);let _=reader.get_mut().write_all(if ok {b"1"}else{b"0"}).await;}
          }).await;});
         },_=workers.join_next(),if !workers.is_empty()=>{}
        }
    }
}
