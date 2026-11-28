#![cfg(target_os = "linux")]
//! The CLI reads the secrets from the keyring and from the file (plan B5, T2; D25, G6). A fake daemon takes the `connect`
//! call and hands its parameters to the test: that is where the key material shows up.
//!
//! `the_cli_without_a_session_bus_reads_the_file_and_stays_quiet` runs with `cargo test`. The two others need the Secret
//! Service of `scripts/gui.sh keyring` (`#[ignore]`, and they **fail** under the wrong `GALLERIA_KEYRING_CASE`).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::{ConnectParams, Status};
use galleria_profile::{FileSecretStore, KeyringSecretStore, ProfileStore, SecretStore};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};

const OVPN: &str = "client\nremote vpn.esempio-ufficio.it 1194\n<key>\nSEED-SECRET-KEY\n</key>\n<tls-auth>\nSEED-SECRET-TLS\n</tls-auth>\n";

/// Takes one `connect` and answers it; returns what it was given.
async fn fake_daemon(listener: UnixListener) -> Option<ConnectParams> {
    let (stream, _) = tokio::time::timeout(Duration::from_secs(20), listener.accept())
        .await
        .ok()?
        .ok()?;
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    let mut seen = None;
    while let Ok(Ok(Some(line))) =
        tokio::time::timeout(Duration::from_secs(5), lines.next_line()).await
    {
        let call: Value = serde_json::from_str(&line).unwrap();
        let result = match call["method"].as_str().unwrap() {
            "hello" => {
                json!({"protocol":1,"daemon_version":"test","backends":[],"status":Status::default()})
            }
            "connect" => {
                seen = Some(serde_json::from_value(call["params"].clone()).unwrap());
                json!({"session": 1, "status": Status::default()})
            }
            _ => json!({}),
        };
        let reply = json!({"jsonrpc":"2.0","id":call["id"],"result":result});
        write
            .write_all(format!("{reply}\n").as_bytes())
            .await
            .unwrap();
        if seen.is_some() {
            break;
        }
    }
    seen
}

struct Imported {
    config: tempfile::TempDir,
    id: uuid::Uuid,
}

/// A profile imported by the CLI, as the CLI does it: everything in the file.
fn import_with_the_cli() -> Imported {
    let config = tempfile::tempdir().unwrap();
    fs::set_permissions(config.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let file = config.path().join("office.ovpn");
    fs::write(&file, OVPN).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_galleria"))
        .args(["import", file.to_str().unwrap()])
        .env("XDG_CONFIG_HOME", config.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let id = String::from_utf8(out.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    Imported { config, id }
}

struct Run {
    ok: bool,
    stdout: String,
    stderr: String,
    params: Option<ConnectParams>,
    took: Duration,
}

/// `galleria connect office --no-wait` against the fake daemon, with the environment `env` applied on top of the
/// inherited one (`None` removes a variable).
async fn connect(config: &Path, env: &[(&str, Option<&str>)]) -> Run {
    let socket = config.join("daemon.sock");
    let daemon = tokio::spawn(fake_daemon(UnixListener::bind(&socket).unwrap()));
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_galleria"));
    cmd.args([
        "--socket",
        socket.to_str().unwrap(),
        "connect",
        "office",
        "--no-wait",
    ])
    .env("XDG_CONFIG_HOME", config)
    .stdin(Stdio::null())
    .kill_on_drop(true);
    for (name, value) in env {
        match value {
            Some(v) => cmd.env(name, v),
            None => cmd.env_remove(name),
        };
    }
    let started = std::time::Instant::now();
    let out = tokio::time::timeout(Duration::from_secs(30), cmd.output())
        .await
        .expect("galleria connect did not finish")
        .unwrap();
    Run {
        ok: out.status.success(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        params: daemon.await.unwrap(),
        took: started.elapsed(),
    }
}

fn blocks(run: &Run) -> (String, String) {
    let galleria_ipc::BackendParams::Openvpn(o) =
        &run.params.as_ref().expect("no connect arrived").backend
    else {
        panic!("not an OpenVPN connect")
    };
    (
        o.secret_blocks["key"].expose().to_string(),
        o.secret_blocks["tls-auth"].expose().to_string(),
    )
}

fn must_be(wanted: &str) {
    let got = std::env::var("GALLERIA_KEYRING_CASE").unwrap_or_default();
    assert_eq!(
        got, wanted,
        "this test needs GALLERIA_KEYRING_CASE={wanted}, set by scripts/gui.sh keyring (got {got:?})"
    );
}

#[tokio::test]
async fn the_cli_without_a_session_bus_reads_the_file_and_stays_quiet() {
    let world = import_with_the_cli();
    let nowhere = tempfile::tempdir().unwrap();
    // no bus address and no `$XDG_RUNTIME_DIR/bus`: there is no keyring, and the CLI must not go looking for one
    let run = connect(
        world.config.path(),
        &[
            ("DBUS_SESSION_BUS_ADDRESS", None),
            ("XDG_RUNTIME_DIR", Some(nowhere.path().to_str().unwrap())),
        ],
    )
    .await;
    assert!(run.ok, "{}", run.stderr);
    assert_eq!(
        blocks(&run),
        ("SEED-SECRET-KEY\n".into(), "SEED-SECRET-TLS\n".into())
    );
    assert_eq!(
        (run.stdout.as_str(), run.stderr.as_str()),
        ("", ""),
        "nothing new on the terminal"
    );
    assert!(run.took < Duration::from_secs(2), "took {:?}", run.took);
}

#[tokio::test]
#[ignore = "needs GNOME Keyring: scripts/gui.sh keyring"]
async fn the_cli_reads_what_the_keyring_holds() {
    must_be("unlocked");
    let world = import_with_the_cli();
    // what the window does: the key goes to the keyring, the other block stays in the file
    let keyring = KeyringSecretStore::open()
        .expect("no Secret Service on the session bus: run through scripts/gui.sh keyring");
    let file = FileSecretStore::open(&world.config.path().join("galleria")).unwrap();
    let key = file.get(&world.id, "openvpn.block.key").unwrap().unwrap();
    keyring.set(&world.id, "openvpn.block.key", &key).unwrap();
    file.delete(&world.id, "openvpn.block.key").unwrap();
    assert!(file.get(&world.id, "openvpn.block.key").unwrap().is_none());
    // the profile is the one the CLI saved; its file does not say where the key is
    assert!(
        ProfileStore::open(&world.config.path().join("galleria"))
            .unwrap()
            .find("office")
            .is_ok()
    );

    let run = connect(world.config.path(), &[]).await;
    keyring.delete_all(&world.id).unwrap();
    assert!(run.ok, "{}", run.stderr);
    assert_eq!(
        blocks(&run),
        ("SEED-SECRET-KEY\n".into(), "SEED-SECRET-TLS\n".into()),
        "the key from the keyring and the block from the file"
    );
    assert_eq!((run.stdout.as_str(), run.stderr.as_str()), ("", ""));
}

#[tokio::test]
#[ignore = "needs a session bus without a Secret Service: scripts/gui.sh keyring"]
async fn the_cli_without_a_keyring_reads_the_file() {
    must_be("absent");
    assert!(
        std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok_and(|a| !a.is_empty()),
        "this case is a bus without the service: DBUS_SESSION_BUS_ADDRESS is not set"
    );
    let world = import_with_the_cli();
    let run = connect(world.config.path(), &[]).await;
    assert!(run.ok, "{}", run.stderr);
    assert_eq!(
        blocks(&run),
        ("SEED-SECRET-KEY\n".into(), "SEED-SECRET-TLS\n".into())
    );
    assert_eq!(
        (run.stdout.as_str(), run.stderr.as_str()),
        ("", ""),
        "nothing new on the terminal"
    );
    assert!(run.took < Duration::from_secs(5), "took {:?}", run.took);
}
