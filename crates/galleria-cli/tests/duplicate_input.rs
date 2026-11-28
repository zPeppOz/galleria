#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::*;
use galleria_profile::*;
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};
#[tokio::test]
async fn queued_duplicate_input_request_does_not_prompt_twice() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let config = dir.path().join("config");
    let profiles = ProfileStore::open(&config.join("galleria")).unwrap();
    profiles
        .save(&Profile::new(
            "office",
            ProtocolConfig::Openvpn(OpenVpnProfile {
                config: "client\n".into(),
            }),
        ))
        .unwrap();
    let socket = dir.path().join("daemon.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut lines = BufReader::new(read).lines();
        let mut needs = Status {
            state: State::NeedsInput,
            session: Some(1),
            input_request: Some(InputRequest {
                request: 1,
                kind: InputKind::Credentials,
                prompt: "Password".into(),
                echo: false,
                expires_in_secs: 120,
            }),
            ..Default::default()
        };
        for method in ["hello", "subscribe", "connect", "status", "provide_input"] {
            let line = lines.next_line().await.unwrap().unwrap();
            let call: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(call["method"], method);
            let result = match method {
                "hello" => {
                    json!({"protocol":1,"daemon_version":"test","backends":[],"status":Status::default()})
                }
                "connect" => json!({"session":1,"status":needs}),
                "status" => {
                    // Status can be returned after its matching event has already been queued.
                    write.write_all(format!("{}\n",json!({"jsonrpc":"2.0","method":"event.state","params":{"status":needs}})).as_bytes()).await.unwrap();
                    json!(needs)
                }
                _ => json!({}),
            };
            write
                .write_all(
                    format!(
                        "{}\n",
                        json!({"jsonrpc":"2.0","id":call["id"],"result":result})
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
        needs.state = State::Connected;
        needs.input_request = None;
        write
            .write_all(
                format!(
                    "{}\n",
                    json!({"jsonrpc":"2.0","method":"event.state","params":{"status":needs}})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    let command = format!(
        "'{}' --socket '{}' connect office --username alice",
        env!("CARGO_BIN_EXE_galleria"),
        socket.display()
    );
    let mut child = tokio::process::Command::new("script")
        .args(["-q", "-e", "-c", &command, "/dev/null"])
        .env("XDG_CONFIG_HOME", config)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut bytes = vec![];
    tokio::time::timeout(Duration::from_secs(3), async {
        while !String::from_utf8_lossy(&bytes).contains("Password: ") {
            bytes.push(stdout.read_u8().await.unwrap());
        }
    })
    .await
    .unwrap();
    stdin.write_all(b"synthetic-password\n").await.unwrap();
    let done = tokio::time::timeout(Duration::from_secs(3), stdout.read_to_end(&mut bytes))
        .await
        .is_ok();
    if !done {
        let _ = child.kill().await;
    }
    let status = child.wait().await.unwrap();
    server.await.unwrap();
    assert!(
        done,
        "CLI prompted again for an input request already answered"
    );
    assert!(status.success());
    let text = String::from_utf8_lossy(&bytes);
    assert_eq!(text.matches("Password: ").count(), 1);
    assert!(text.contains("connected"));
    assert!(!text.contains("synthetic-password"));
}
