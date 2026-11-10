#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleria_ipc::*;
use serde_json::{Value, json};

fn status_json() -> Value {
    serde_json::from_str(r#"{"state":"connected","session":7,"profile":{"id":"3f0c…","name":"Ufficio Napoli"},"protocol":"openvpn",
 "since":"2026-11-10T10:42:03Z","attempt":1,"detail":null,
 "local_addresses":["10.8.0.2"],"remote":"172.31.0.10:1194",
 "net":{"interface":"glr-ovpn0","dns_servers":["10.8.0.1"],"search_domains":["galleria.test"],
        "routes":["0.0.0.0/1","128.0.0.0/1","192.0.2.0/24"],"full_tunnel":true,"ipv6_block":"unreachable"},
 "stats":{"rx_bytes":48213000,"tx_bytes":6120000,"last_handshake":null},
 "input_request":null,"error":null}"#).unwrap()
}
fn wg_json() -> Value {
    serde_json::from_str(
        r#"{"protocol":"wireguard","private_key":"<base64>","addresses":["10.99.0.2/24"],
 "dns":["10.99.0.1"],"search_domains":[],"mtu":null,
 "peers":[{"public_key":"<base64>","preshared_key":null,"endpoint":"vpn.example.org:51820",
           "allowed_ips":["0.0.0.0/0"],"persistent_keepalive":25}]}"#,
    )
    .unwrap()
}
fn ovpn_json() -> Value {
    serde_json::from_str(
        r#"{"protocol":"openvpn","config":"client\nremote 172.31.0.10 1194\n…<ca>…</ca>\n",
 "secret_blocks":{"key":"-----BEGIN PRIVATE KEY-----…","tls-auth":"…"},
 "username":"alice","password":"…","otp":null,"key_passphrase":null}"#,
    )
    .unwrap()
}
fn connect_json() -> Value {
    json!({"profile":{"id":"profile-1","name":"Ufficio Napoli"},"replace":false,"backend":wg_json(),
           "options":{"ipv6_block":true,"ignore_pushed_routes":false,"reconnect":{"enabled":true}}})
}
fn hello_json() -> Value {
    json!({"protocol":1,"daemon_version":"0.1.0","backends":[
        {"protocol":"wireguard","available":true,"version":null,"unavailable_reason":null,
         "capabilities":{"handshake":true,"interactive_auth":false,"server_pushed_config":false}},
        {"protocol":"openvpn","available":true,"version":"2.6.19","unavailable_reason":null,
         "capabilities":{"handshake":false,"interactive_auth":true,"server_pushed_config":true}}],"status":status_json()})
}
#[test]
fn hello_request_matches_golden_json() {
    let r=decode_line(br#"{"jsonrpc":"2.0","id":1,"method":"hello","params":{"protocol":1,"client":{"name":"galleria-cli","version":"0.1.0"}}}"#).unwrap();
    let Call::Hello(p) = parse_call(&r.method, r.params).unwrap() else {
        panic!("expected hello")
    };
    assert_eq!(p.protocol, 1);
    assert_eq!(p.client.name, "galleria-cli");
}
#[test]
fn status_golden_json_deserializes() {
    let s: Status = serde_json::from_value(status_json()).unwrap();
    assert_eq!(s.state, State::Connected);
    assert_eq!(
        s.net.as_ref().unwrap().ipv6_block,
        Ipv6BlockInfo::Unreachable
    );
    assert_eq!(s.stats.as_ref().unwrap().rx_bytes, 48_213_000);
    assert_eq!(serde_json::to_value(s).unwrap(), status_json());
}
#[test]
fn backend_params_are_tagged_by_protocol() {
    for (v, is_wg) in [(wg_json(), true), (ovpn_json(), false)] {
        let b: BackendParams = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(matches!(b, BackendParams::Wireguard(_)), is_wg);
        assert_eq!(serde_json::to_value(b).unwrap(), v);
    }
}
#[test]
fn connect_rejects_unknown_field() {
    let mut v = connect_json();
    v["evil"] = json!(1);
    assert_eq!(parse_call("connect", v).unwrap_err().code, -32602);
}
#[test]
fn unknown_method_is_method_not_found() {
    assert_eq!(parse_call("reboot", json!({})).unwrap_err().code, -32601);
}
#[test]
fn decode_rejects_batch_array() {
    assert_eq!(decode_line(b"[]").unwrap_err().code, -32600);
}
#[test]
fn decode_rejects_request_without_id() {
    assert_eq!(
        decode_line(br#"{"jsonrpc":"2.0","method":"status","params":{}}"#)
            .unwrap_err()
            .code,
        -32600
    );
}
#[test]
fn decode_rejects_invalid_json() {
    assert_eq!(decode_line(b"{").unwrap_err().code, -32700);
}
#[tokio::test]
async fn read_frame_accepts_line_at_limit() {
    let mut line = vec![b'x'; MAX_MESSAGE_BYTES];
    line[MAX_MESSAGE_BYTES - 1] = b'\n';
    let frame = read_frame(&mut line.as_slice(), MAX_MESSAGE_BYTES)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame.len(), MAX_MESSAGE_BYTES);
    assert!(frame.capacity() <= MAX_MESSAGE_BYTES);
}
#[tokio::test]
async fn read_frame_rejects_line_over_limit() {
    let mut line = vec![b'x'; MAX_MESSAGE_BYTES + 1];
    line[MAX_MESSAGE_BYTES] = b'\n';
    assert!(matches!(
        read_frame(&mut line.as_slice(), MAX_MESSAGE_BYTES).await,
        Err(FrameError::TooLarge)
    ));
}
#[test]
fn negotiate_accepts_1_and_rejects_others() {
    assert_eq!(negotiate(1).unwrap(), 1);
    for n in [0, 2, u32::MAX] {
        let d = negotiate(n).unwrap_err().data.unwrap();
        assert_eq!(d["code"], "incompatible_protocol");
        assert_eq!(d["daemon_min"], 1);
        assert_eq!(d["daemon_max"], 1);
        assert_eq!(d["daemon_version"], "0.1.0");
    }
}
#[test]
fn secret_debug_is_redacted() {
    let s = Secret::new("hunter2");
    assert_eq!(format!("{s:?}"), "Secret(..)");
    assert_eq!(s.expose(), "hunter2");
    let encoded = serde_json::to_string(&s).unwrap();
    assert_eq!(
        serde_json::from_str::<Secret>(&encoded).unwrap().expose(),
        "hunter2"
    );
}
#[test]
fn client_side_types_ignore_unknown_fields() {
    let mut v = hello_json();
    v["future"] = json!(1);
    v["status"]["future"] = json!(true);
    let h: HelloResult = serde_json::from_value(v).unwrap();
    assert_eq!(h.protocol, 1);
    assert_eq!(serde_json::to_value(h).unwrap(), hello_json());
}
#[test]
fn error_codes_are_stable_strings() {
    for (code, s) in [
        (ErrorCode::HelloRequired, "hello_required"),
        (ErrorCode::IncompatibleProtocol, "incompatible_protocol"),
        (ErrorCode::MessageTooLarge, "message_too_large"),
        (ErrorCode::TooManyClients, "too_many_clients"),
        (ErrorCode::AlreadyActive, "already_active"),
        (ErrorCode::NotActive, "not_active"),
        (ErrorCode::InvalidConfig, "invalid_config"),
        (ErrorCode::ProfileRejected, "profile_rejected"),
        (ErrorCode::BackendUnavailable, "backend_unavailable"),
        (ErrorCode::InputNotExpected, "input_not_expected"),
        (ErrorCode::Internal, "internal"),
    ] {
        assert_eq!(code.as_str(), s);
        let e = code.to_rpc("Request rejected");
        assert_eq!(e.code, -32000);
        assert_eq!(e.data.unwrap()["code"], s);
    }
    for (code, s) in [
        (SessionErrorCode::AuthFailed, "auth_failed"),
        (SessionErrorCode::InputTimeout, "input_timeout"),
        (SessionErrorCode::ServerUnreachable, "server_unreachable"),
        (SessionErrorCode::TunnelSetupFailed, "tunnel_setup_failed"),
        (SessionErrorCode::NetSetupFailed, "net_setup_failed"),
        (SessionErrorCode::DnsUnavailable, "dns_unavailable"),
        (
            SessionErrorCode::UnsupportedChallenge,
            "unsupported_challenge",
        ),
        (SessionErrorCode::BackendExited, "backend_exited"),
    ] {
        assert_eq!(serde_json::to_value(code).unwrap(), json!(s));
    }
}
#[test]
fn all_method_params_and_notifications_match_the_wire_contract() {
    assert!(matches!(
        parse_call("connect", connect_json()).unwrap(),
        Call::Connect(_)
    ));
    assert!(matches!(
        parse_call("status", json!({})).unwrap(),
        Call::Status
    ));
    assert!(matches!(
        parse_call("disconnect", json!({})).unwrap(),
        Call::Disconnect
    ));
    assert!(matches!(
        parse_call(
            "provide_input",
            json!({"session":7,"request":3,"values":{"otp":"test"}})
        )
        .unwrap(),
        Call::ProvideInput(_)
    ));
    assert!(matches!(
        parse_call("subscribe", json!({"stats":true,"logs":false})).unwrap(),
        Call::Subscribe(_)
    ));
    assert!(matches!(
        parse_call("logs", json!({"after":null,"limit":1000})).unwrap(),
        Call::Logs(_)
    ));
    assert_eq!(
        parse_call("logs", json!({"after":null,"limit":1001}))
            .unwrap_err()
            .code,
        -32602
    );
    let status: Status = serde_json::from_value(status_json()).unwrap();
    let event: Value = serde_json::from_slice(&notification(&Event::State(status))).unwrap();
    assert_eq!(
        event,
        json!({"jsonrpc":"2.0","method":"event.state","params":{"status":status_json()}})
    );
    let ok: Value =
        serde_json::from_slice(&response_ok(&RequestId::Str("x".into()), hello_json())).unwrap();
    assert_eq!(ok, json!({"jsonrpc":"2.0","id":"x","result":hello_json()}));
    let err = ErrorCode::HelloRequired.to_rpc("Hello required");
    let wire: Value = serde_json::from_slice(&response_err(None, &err)).unwrap();
    assert_eq!(wire["id"], Value::Null);
    assert_eq!(wire["error"]["data"]["code"], "hello_required");
}
#[test]
fn unknown_fields_are_rejected_at_every_request_boundary() {
    for pointer in [
        "/profile",
        "/backend",
        "/backend/peers/0",
        "/options",
        "/options/reconnect",
    ] {
        let mut v = connect_json();
        v.pointer_mut(pointer).unwrap()["evil"] = json!(1);
        assert_eq!(
            parse_call("connect", v).unwrap_err().code,
            -32602,
            "{pointer}"
        );
    }
    for (method, v) in [
        ("status", json!({"evil":1})),
        ("disconnect", json!({"evil":1})),
        (
            "provide_input",
            json!({"session":1,"request":1,"values":{"evil":1}}),
        ),
        ("subscribe", json!({"stats":false,"logs":false,"evil":1})),
        ("logs", json!({"after":null,"limit":1,"evil":1})),
    ] {
        assert_eq!(parse_call(method, v).unwrap_err().code, -32602);
    }
    for s in [
        r#"{"jsonrpc":"1.0","id":1,"method":"status","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":null,"method":"status","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":1.5,"method":"status","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"status","params":{},"evil":1}"#,
    ] {
        assert_eq!(decode_line(s.as_bytes()).unwrap_err().code, -32600);
    }
}
#[tokio::test]
async fn frame_reader_does_not_consume_following_lines() {
    let mut r = &b"one\ntwo\n"[..];
    assert_eq!(read_frame(&mut r, 4).await.unwrap().unwrap(), b"one\n");
    assert_eq!(read_frame(&mut r, 4).await.unwrap().unwrap(), b"two\n");
    assert!(read_frame(&mut r, 4).await.unwrap().is_none());
    assert!(matches!(
        read_frame(&mut &b"half"[..], 8).await,
        Err(FrameError::Io(_))
    ));
}

#[test]
fn positional_arrays_are_never_request_objects() {
    for wire in [
        br#"["2.0",1,"status",{}]"#.as_slice(),
        br#"[{"jsonrpc":"2.0","id":1,"method":"status","params":{}}]"#.as_slice(),
    ] {
        assert_eq!(decode_line(wire).unwrap_err().code, -32600);
    }
}
#[test]
fn parameter_structs_require_objects_at_every_level() {
    for (method, value) in [
        ("status", json!([])),
        ("disconnect", json!([])),
        ("hello", json!([1, ["client", "1"]])),
        ("hello", json!({"protocol":1,"client":["client","1"]})),
        ("subscribe", json!([true, false])),
        ("logs", json!([null, 1])),
        ("provide_input", json!([1, 1, [null, null, null, null]])),
        (
            "provide_input",
            json!({"session":1,"request":1,"values":[null,null,null,null]}),
        ),
    ] {
        assert_eq!(
            parse_call(method, value).unwrap_err().code,
            -32602,
            "{method}"
        );
    }
    for (pointer, replacement) in [
        ("/profile", json!(["id", "name"])),
        ("/options", json!([true, false, [true]])),
        ("/options/reconnect", json!([true])),
        (
            "/backend/peers/0",
            json!(["public-key", null, "endpoint:1", ["0.0.0.0/0"], 25]),
        ),
    ] {
        let mut value = connect_json();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_eq!(
            parse_call("connect", value).unwrap_err().code,
            -32602,
            "{pointer}"
        );
    }
}
#[test]
fn response_serialization_failure_preserves_request_id() {
    struct Failing;
    impl serde::Serialize for Failing {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("Synthetic serializer failure"))
        }
    }
    for id in [RequestId::Num(42), RequestId::Str("request-42".into())] {
        let frame = response_ok(&id, Failing);
        assert_eq!(frame.last(), Some(&b'\n'));
        let value: Value = serde_json::from_slice(&frame).unwrap();
        assert_eq!(value["id"], serde_json::to_value(&id).unwrap());
        assert_eq!(value["error"]["data"]["code"], "internal");
        assert!(
            !String::from_utf8(frame)
                .unwrap()
                .contains("Synthetic serializer failure")
        );
    }
}

#[test]
fn hello_with_future_fields_reports_incompatible_protocol() {
    let value = json!({"protocol":2,"future":true,"client":{"name":"future-client","version":"2.0","future":true}});
    let result = parse_call("hello", value).and_then(|call| match call {
        Call::Hello(p) => negotiate(p.protocol),
        _ => panic!("expected hello"),
    });
    let error = result.unwrap_err();
    assert_eq!(
        error.data.as_ref().and_then(|d| d.get("code")),
        Some(&json!("incompatible_protocol"))
    );
    assert_eq!(error.data.as_ref().unwrap()["daemon_min"], 1);
    assert_eq!(error.data.as_ref().unwrap()["daemon_max"], 1);
    assert!(error.message.contains("update Galleria"));
    assert!(
        parse_call(
            "hello",
            json!({"protocol":1,"extra":true,"client":{"name":"test","version":"1","extra":true}})
        )
        .is_ok()
    );
}

// Exercise repeated fill_buf/consume cycles rather than an in-memory slice returned in one read.
struct Fragmented {
    bytes: Vec<u8>,
    read: usize,
}
impl tokio::io::AsyncRead for Fragmented {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        out: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let n = 17.min(out.remaining()).min(self.bytes.len() - self.read);
        out.put_slice(&self.bytes[self.read..self.read + n]);
        self.read += n;
        std::task::Poll::Ready(Ok(()))
    }
}
#[tokio::test]
async fn fragmented_line_enforces_limit_without_geometric_allocation() {
    for extra in [0, 1] {
        let mut bytes = vec![b'x'; MAX_MESSAGE_BYTES + extra];
        *bytes.last_mut().unwrap() = b'\n';
        let mut reader = tokio::io::BufReader::with_capacity(17, Fragmented { bytes, read: 0 });
        let result = read_frame(&mut reader, MAX_MESSAGE_BYTES).await;
        if extra == 0 {
            let frame = result.unwrap().unwrap();
            assert_eq!(frame.len(), MAX_MESSAGE_BYTES);
            assert!(frame.capacity() <= MAX_MESSAGE_BYTES);
        } else {
            assert!(matches!(result, Err(FrameError::TooLarge)));
            assert!(reader.get_ref().read <= MAX_MESSAGE_BYTES + 17);
        }
    }
}
