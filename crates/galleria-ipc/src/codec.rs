use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub fn decode_line(line: &[u8]) -> Result<Request, RpcError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct WireRequest {
        jsonrpc: String,
        id: RequestId,
        method: String,
        params: serde_json::Value,
    }
    // Separate syntax errors from structurally invalid JSON-RPC objects.
    let value: serde_json::Value =
        serde_json::from_slice(line).map_err(|_| RpcError::standard(-32700, "Parse error"))?;
    if !value.is_object() {
        return Err(RpcError::standard(-32600, "Invalid request"));
    }
    let r: WireRequest =
        serde_json::from_value(value).map_err(|_| RpcError::standard(-32600, "Invalid request"))?;
    if r.jsonrpc != "2.0" {
        return Err(RpcError::standard(-32600, "Invalid request"));
    }
    Ok(Request {
        id: r.id,
        method: r.method,
        params: r.params,
    })
}
pub fn encode_line<T: Serialize>(msg: &T) -> Vec<u8> {
    let mut out=match serde_json::to_vec(msg) {
        Ok(out)=>out,
        Err(_)=>br#"{"jsonrpc":"2.0","id":null,"error":{"code":-32000,"message":"Serialization failed","data":{"code":"internal"}}}"#.to_vec(),
    };
    out.push(b'\n');
    out
}
pub fn response_ok(id: &RequestId, result: impl Serialize) -> Vec<u8> {
    match serde_json::to_value(result) {
        Ok(result) => encode_line(&json!({"jsonrpc":"2.0","id":id,"result":result})),
        Err(_) => response_err(
            Some(id),
            &ErrorCode::Internal.to_rpc("Serialization failed"),
        ),
    }
}
pub fn response_err(id: Option<&RequestId>, err: &RpcError) -> Vec<u8> {
    encode_line(&json!({"jsonrpc":"2.0","id":id,"error":err}))
}
pub fn notification(event: &Event) -> Vec<u8> {
    match event {
        Event::State(s) => {
            encode_line(&json!({"jsonrpc":"2.0","method":"event.state","params":{"status":s}}))
        }
        Event::Stats(s) => encode_line(&json!({"jsonrpc":"2.0","method":"event.stats","params":s})),
        Event::Log(l) => encode_line(&json!({"jsonrpc":"2.0","method":"event.log","params":l})),
    }
}
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("Message too large")]
    TooLarge,
    #[error("Frame I/O failed")]
    Io(#[from] std::io::Error),
}
pub async fn read_frame<R: AsyncBufRead + Unpin>(
    r: &mut R,
    max: usize,
) -> Result<Option<Vec<u8>>, FrameError> {
    let mut out = Vec::new();
    loop {
        let buf = r.fill_buf().await?;
        if buf.is_empty() {
            return if out.is_empty() {
                Ok(None)
            } else {
                Err(FrameError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Unterminated frame",
                )))
            };
        }
        let end = buf.iter().position(|b| *b == b'\n').map(|n| n + 1);
        let count = end.unwrap_or(buf.len());
        if count > max.saturating_sub(out.len()) {
            return Err(FrameError::TooLarge);
        }
        // Exact reservations prevent Vec's geometric growth from exceeding the frame limit.
        out.reserve_exact(count);
        out.extend_from_slice(&buf[..count]);
        r.consume(count);
        if end.is_some() {
            return Ok(Some(out));
        }
        if out.len() == max {
            return Err(FrameError::TooLarge);
        }
    }
}
