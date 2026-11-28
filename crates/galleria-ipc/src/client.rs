use crate::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::VecDeque, io, path::Path};
use tokio::io::{AsyncWriteExt, BufReader};
#[cfg(unix)]
type Transport = tokio::net::UnixStream;
/// Windows has no transport yet (the named pipe is P2): the type only has to exist, `connect` never makes one.
#[cfg(windows)]
type Transport = tokio::io::DuplexStream;
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("IPC I/O failed")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Rpc(RpcError),
    #[error("Daemon closed the connection")]
    Closed,
    #[error("Permission denied; join the galleria group or run as root")]
    PermissionDenied,
}
pub struct Client {
    stream: BufReader<Transport>,
    next_id: u64,
    events: VecDeque<Event>,
}
impl Client {
    #[cfg(windows)]
    pub async fn connect(_path: &Path) -> Result<Self, ClientError> {
        Err(ClientError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "galleria: the Windows transport is not there yet",
        )))
    }
    #[cfg(unix)]
    pub async fn connect(path: &Path) -> Result<Self, ClientError> {
        let stream = Transport::connect(path).await.map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                ClientError::PermissionDenied
            } else {
                ClientError::Io(e)
            }
        })?;
        Ok(Self {
            stream: BufReader::new(stream),
            next_id: 0,
            events: VecDeque::new(),
        })
    }
    pub async fn hello(&mut self, name: &str, version: &str) -> Result<HelloResult, ClientError> {
        self.call(
            "hello",
            &HelloParams {
                protocol: PROTOCOL_VERSION,
                client: ClientInfo {
                    name: name.into(),
                    version: version.into(),
                },
            },
        )
        .await
    }
    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &mut self,
        method: &str,
        params: &P,
    ) -> Result<R, ClientError> {
        self.next_id += 1;
        let id = self.next_id;
        let frame = encode_line(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        if frame.len() > MAX_MESSAGE_BYTES {
            return Err(invalid_data());
        }
        self.stream.get_mut().write_all(&frame).await?;
        loop {
            let value = self.read().await?;
            if value.get("method").is_some() {
                if let Some(event) = event(&value) {
                    self.events.push_back(event);
                }
                continue;
            }
            // Admission errors have a null id because the server has not read a request yet.
            if value.get("id") != Some(&json!(id)) && value.get("id") != Some(&Value::Null) {
                return Err(invalid_data());
            }
            if let Some(error) = value.get("error") {
                return Err(ClientError::Rpc(
                    serde_json::from_value(error.clone()).map_err(|_| invalid_data())?,
                ));
            }
            return serde_json::from_value(value.get("result").cloned().ok_or_else(invalid_data)?)
                .map_err(|_| invalid_data());
        }
    }
    pub async fn next_event(&mut self) -> Option<Event> {
        if let Some(event) = self.events.pop_front() {
            return Some(event);
        }
        loop {
            let value = self.read().await.ok()?;
            if let Some(event) = event(&value) {
                return Some(event);
            }
        }
    }
    async fn read(&mut self) -> Result<Value, ClientError> {
        let bytes = read_frame(&mut self.stream, MAX_MESSAGE_BYTES)
            .await
            .map_err(|e| match e {
                FrameError::Io(e) => ClientError::Io(e),
                FrameError::TooLarge => invalid_data(),
            })?
            .ok_or(ClientError::Closed)?;
        serde_json::from_slice(&bytes).map_err(|_| invalid_data())
    }
}
fn invalid_data() -> ClientError {
    ClientError::Io(io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid daemon frame",
    ))
}
fn event(value: &Value) -> Option<Event> {
    let params = value.get("params")?;
    match value.get("method")?.as_str()? {
        "event.state" => serde_json::from_value(params.get("status")?.clone())
            .ok()
            .map(Event::State),
        "event.stats" => serde_json::from_value(params.clone())
            .ok()
            .map(Event::Stats),
        "event.log" => serde_json::from_value(params.clone()).ok().map(Event::Log),
        _ => None,
    }
}
