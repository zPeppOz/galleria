use crate::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

#[derive(Debug)]
pub enum Call {
    Hello(HelloParams),
    Status,
    Connect(ConnectParams),
    Disconnect,
    ProvideInput(ProvideInputParams),
    Subscribe(SubscribeParams),
    Logs(LogsParams),
}
fn params<T: DeserializeOwned>(value: Value) -> Result<T, RpcError> {
    if !value.is_object() {
        return Err(RpcError::standard(-32602, "Invalid params"));
    }
    serde_json::from_value(value).map_err(|_| RpcError::standard(-32602, "Invalid params"))
}
fn object_fields(value: &Value, paths: &[&str]) -> Result<(), RpcError> {
    if paths
        .iter()
        .any(|path| !value.pointer(path).is_some_and(Value::is_object))
    {
        return Err(RpcError::standard(-32602, "Invalid params"));
    }
    Ok(())
}
pub fn parse_call(method: &str, value: Value) -> Result<Call, RpcError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    match method {
        "hello" => {
            // Negotiate before interpreting the version-specific request schema.
            let protocol = value
                .get("protocol")
                .and_then(Value::as_u64)
                .and_then(|p| u32::try_from(p).ok());
            negotiate(protocol.unwrap_or(0))?;
            object_fields(&value, &["/client"])?;
            params(value).map(Call::Hello)
        }
        "status" => {
            let _: Empty = params(value)?;
            Ok(Call::Status)
        }
        "connect" => {
            object_fields(
                &value,
                &["/profile", "/backend", "/options", "/options/reconnect"],
            )?;
            if value
                .pointer("/backend/peers")
                .and_then(Value::as_array)
                .is_some_and(|peers| peers.iter().any(|peer| !peer.is_object()))
            {
                return Err(RpcError::standard(-32602, "Invalid params"));
            }
            params(value).map(Call::Connect)
        }
        "disconnect" => {
            let _: Empty = params(value)?;
            Ok(Call::Disconnect)
        }
        "provide_input" => {
            object_fields(&value, &["/values"])?;
            params(value).map(Call::ProvideInput)
        }
        "subscribe" => params(value).map(Call::Subscribe),
        "logs" => {
            let p: LogsParams = params(value)?;
            if p.limit > 1000 {
                return Err(RpcError::standard(-32602, "Invalid params"));
            }
            Ok(Call::Logs(p))
        }
        _ => Err(RpcError::standard(-32601, "Method not found")),
    }
}
pub fn negotiate(client_protocol: u32) -> Result<u32, RpcError> {
    if (PROTOCOL_MIN..=PROTOCOL_MAX).contains(&client_protocol) {
        return Ok(client_protocol);
    }
    let mut e = ErrorCode::IncompatibleProtocol
        .to_rpc("Incompatible protocol; update Galleria or restart the daemon");
    e.data = Some(
        json!({"code":"incompatible_protocol","daemon_min":PROTOCOL_MIN,"daemon_max":PROTOCOL_MAX,"daemon_version":env!("CARGO_PKG_VERSION")}),
    );
    Err(e)
}
