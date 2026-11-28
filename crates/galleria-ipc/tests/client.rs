#![cfg(all(feature = "client", unix))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::{client::*, *};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixListener,
};
#[tokio::test]
async fn unknown_notifications_are_ignored_and_interleaved_events_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ipc.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let request = decode_line(
            &read_frame(&mut stream, MAX_MESSAGE_BYTES)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        stream
            .get_mut()
            .write_all(&encode_line(
                &json!({"jsonrpc":"2.0","method":"event.future","params":{"new":true}}),
            ))
            .await
            .unwrap();
        stream
            .get_mut()
            .write_all(&notification(&Event::State(Status::default())))
            .await
            .unwrap();
        stream
            .get_mut()
            .write_all(&response_ok(&request.id, json!({"ok":true,"future":true})))
            .await
            .unwrap();
    });
    let mut client = Client::connect(&path).await.unwrap();
    let value: Value = client.call("status", &json!({})).await.unwrap();
    assert_eq!(value["ok"], true);
    assert!(
        matches!(client.next_event().await,Some(Event::State(s)) if s.state==State::Disconnected)
    );
    server.await.unwrap();
    assert!(client.next_event().await.is_none());
}
