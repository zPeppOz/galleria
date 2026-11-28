#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::*;
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, unix::OwnedWriteHalf},
};

fn entry(seq: u64) -> LogEntry {
    LogEntry {
        seq,
        ts: seq.to_string(),
        level: LogLevel::Info,
        source: LogSource::Daemon,
        message: format!("line-{seq}"),
    }
}

async fn send(write: &mut OwnedWriteHalf, value: Value) {
    write
        .write_all(format!("{value}\n").as_bytes())
        .await
        .unwrap();
}

/// Fake daemon with the `logs` semantics of the real buffer: entries after `after`,
/// oldest first, at most 1000 per page. With `follow`, new lines are pushed while
/// the CLI is still paging, and once more after the last page.
async fn serve(listener: UnixListener, lines: u64, follow: bool) {
    let (stream, _) = listener.accept().await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read).lines();
    let mut buffer: Vec<LogEntry> = (1..=lines).map(entry).collect();
    let mut pages = 0;
    while let Ok(Ok(Some(line))) =
        tokio::time::timeout(Duration::from_secs(2), reader.next_line()).await
    {
        let call: Value = serde_json::from_str(&line).unwrap();
        let result = match call["method"].as_str().unwrap() {
            "hello" => {
                json!({"protocol":1,"daemon_version":"test","backends":[],"status":Status::default()})
            }
            "logs" => {
                let params: LogsParams = serde_json::from_value(call["params"].clone()).unwrap();
                let page: Vec<_> = buffer
                    .iter()
                    .filter(|e| params.after.is_none_or(|s| e.seq > s))
                    .take(params.limit.min(1000))
                    .cloned()
                    .collect();
                let last = page
                    .last()
                    .map(|e| e.seq)
                    .unwrap_or(params.after.unwrap_or(lines));
                json!({"entries": page, "last": last})
            }
            _ => json!({}),
        };
        let partial = result["entries"].as_array().is_some_and(|e| e.len() < 1000);
        send(
            &mut write,
            json!({"jsonrpc":"2.0","id":call["id"],"result":result}),
        )
        .await;
        if call["method"] == "logs" {
            pages += 1;
            if follow && pages == 1 {
                // Queued while paging: an old line and one the next page will also return.
                let new = entry(buffer.len() as u64 + 1);
                buffer.push(new.clone());
                for e in [entry(lines - 1), new] {
                    send(
                        &mut write,
                        json!({"jsonrpc":"2.0","method":"event.log","params":e}),
                    )
                    .await;
                }
            }
            if partial {
                if follow {
                    let new = entry(buffer.len() as u64 + 1);
                    send(
                        &mut write,
                        json!({"jsonrpc":"2.0","method":"event.log","params":new}),
                    )
                    .await;
                }
                break;
            }
        }
    }
}

async fn run_logs(lines: u64, follow: bool) -> (Option<i32>, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("daemon.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(serve(listener, lines, follow));
    let mut args = vec!["--socket".to_string(), socket.display().to_string()];
    args.push("logs".into());
    if follow {
        args.push("-f".into());
    }
    let output = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new(env!("CARGO_BIN_EXE_galleria"))
            .args(&args)
            .env("XDG_CONFIG_HOME", dir.path())
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("galleria logs did not finish")
    .unwrap();
    server.await.unwrap();
    let messages = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|l| l.rsplit(' ').next().unwrap().to_string())
        .collect();
    (output.status.code(), messages)
}

/// Every line from 1 to `n` exactly once and in order, reported compactly.
fn assert_lines(messages: &[String], n: u64) {
    let expected: Vec<_> = (1..=n).map(|s| format!("line-{s}")).collect();
    let first_diff = messages.iter().zip(&expected).position(|(a, b)| a != b);
    assert!(
        first_diff.is_none() && messages.len() == expected.len(),
        "got {} lines (last {:?}), wanted {n}; first difference at index {first_diff:?}: {:?}",
        messages.len(),
        messages.last(),
        first_diff.map(|i| &messages[i]),
    );
}

#[tokio::test]
async fn logs_pages_through_the_whole_buffer_to_the_latest_line() {
    let (code, messages) = run_logs(2300, false).await;
    assert_eq!(code, Some(0));
    assert_lines(&messages, 2300);
}

#[tokio::test]
async fn follow_skips_nothing_past_the_first_page_and_prints_each_line_once() {
    let (_, messages) = run_logs(2300, true).await;
    // 2300 buffered, 2301 pushed while paging, 2302 pushed after the last page.
    assert_lines(&messages, 2302);
}

#[tokio::test]
async fn logs_on_an_exact_page_boundary_terminates() {
    let (code, messages) = run_logs(1000, false).await;
    assert_eq!(code, Some(0));
    assert_lines(&messages, 1000);
}
