#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_e2e::*;
use galleria_ipc::*;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};
fn spawn_as(user: &str, args: &[&str]) -> Child {
    assert_lab();
    Command::new("runuser")
        .args(["-u", user, "--", "/usr/local/bin/galleria"])
        .args(args)
        .env("LC_ALL", "C")
        .env("XDG_CONFIG_HOME", format!("/home/{user}/.config"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}
fn cli(args: &[&str], input: &[u8]) -> Output {
    let mut child = spawn_as("alice", args);
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
fn success(args: &[&str], input: &[u8]) -> String {
    let out = cli(args, input);
    assert!(
        out.status.success(),
        "CLI failed ({:?}): {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}
async fn clean() -> client::Client {
    let mut c = client().await;
    disconnect(&mut c).await;
    checked(
        "rm -rf /home/alice/.config/galleria /home/alice/incoming; install -d -o alice -g alice -m700 /home/alice/incoming",
    );
    c
}
fn import(file: &str, name: &str) -> String {
    let path = format!("/home/alice/incoming/{file}");
    std::fs::write(&path, lab_file(file)).unwrap();
    checked(&format!("chown alice:alice '{path}'; chmod 600 '{path}'"));
    success(&["import", &path, "--name", name], &[])
        .split_whitespace()
        .next()
        .unwrap()
        .into()
}
#[tokio::test]
#[ignore]
async fn cli_wireguard_roundtrip() {
    let mut c = clean().await;
    let before = residue();
    import("wg-split.conf", "office");
    assert!(success(&["list"], &[]).contains("office"));
    success(&["connect", "office"], &[]);
    assert_eq!(success(&["status"], &[]).trim(), "connected");
    checked("ping -c 2 -W 2 10.99.0.1");
    success(&["disconnect"], &[]);
    wait_state(&mut c, State::Disconnected, Duration::from_secs(5)).await;
    assert_eq!(residue(), before);
}
#[tokio::test]
#[ignore]
async fn cli_openvpn_with_password_stdin() {
    let mut c = clean().await;
    import("base.ovpn", "base");
    success(
        &["connect", "base", "--username", "alice", "--password-stdin"],
        b"alice-secret\n",
    );
    let s: Status = serde_json::from_str(&success(&["status", "--json"], &[])).unwrap();
    assert_eq!(s.state, State::Connected);
    checked("ping -c 2 -W 2 10.8.0.1");
    disconnect(&mut c).await;
}
fn code() -> String {
    checked("oathtool --totp -b JBSWY3DPEHPK3PXP")
}
#[tokio::test]
#[ignore]
async fn cli_otp_from_stdin() {
    let mut c = clean().await;
    import("otp.ovpn", "otp");
    success(
        &[
            "connect",
            "otp",
            "--username",
            "bob",
            "--password-stdin",
            "--otp-stdin",
        ],
        format!("bob-secret\n{}\n", code()).as_bytes(),
    );
    assert_eq!(success(&["status"], &[]).trim(), "connected");
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn cli_secrets_never_appear_in_argv() {
    let mut c = clean().await;
    for (file, name, user, password) in [
        ("base.ovpn", "base", "alice", "alice-secret"),
        ("otp.ovpn", "otp", "bob", "bob-secret"),
    ] {
        import(file, name);
        let otp = code();
        let mut args = vec!["connect", name, "--username", user, "--password-stdin"];
        if name == "otp" {
            args.push("--otp-stdin");
        }
        let mut child = spawn_as("alice", &args);
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{password}").unwrap();
        if name == "otp" {
            writeln!(stdin, "{otp}").unwrap();
        }
        drop(stdin);
        let mut samples = 0;
        while child.try_wait().unwrap().is_none() {
            let processes = checked("ps -eo pid,args");
            assert!(!processes.contains(password));
            assert!(!processes.contains(&otp));
            let pids = sh(&format!("pgrep -P {}", child.id()));
            for pid in String::from_utf8_lossy(&pids.stdout).split_whitespace() {
                if let Ok(argv) = std::fs::read(format!("/proc/{pid}/cmdline")) {
                    let argv = String::from_utf8_lossy(&argv);
                    assert!(!argv.contains(password));
                    assert!(!argv.contains(&otp));
                    samples += 1;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(child.wait_with_output().unwrap().status.success());
        assert!(samples > 0);
        disconnect(&mut c).await;
    }
}
#[tokio::test]
#[ignore]
async fn cli_wrong_password_exits_2() {
    let mut c = clean().await;
    import("base.ovpn", "base");
    let out = cli(
        &["connect", "base", "--username", "alice", "--password-stdin"],
        b"wrong\n",
    );
    assert_eq!(out.status.code(), Some(2));
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn cli_unreachable_exits_4() {
    let mut c = clean().await;
    import("unreachable.ovpn", "unreachable");
    let now = Instant::now();
    let out = cli(
        &[
            "connect",
            "unreachable",
            "--username",
            "alice",
            "--password-stdin",
        ],
        b"alice-secret\n",
    );
    assert_eq!(out.status.code(), Some(4));
    assert!(now.elapsed() < Duration::from_secs(45));
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn cli_connect_while_active_exits_1_and_replace_works() {
    let mut c = clean().await;
    import("wg-split.conf", "office");
    success(&["connect", "office"], &[]);
    let first: Status = c.call("status", &serde_json::json!({})).await.unwrap();
    assert_eq!(cli(&["connect", "office"], &[]).status.code(), Some(1));
    success(&["connect", "office", "--replace"], &[]);
    let second: Status = c.call("status", &serde_json::json!({})).await.unwrap();
    assert_ne!(first.session, second.session);
    assert_eq!(second.state, State::Connected);
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn cli_non_member_exits_5() {
    clean().await;
    let out = spawn_as("mallory", &["status"]).wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(5));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("galleria"));
    assert!(err.contains("usermod -aG"));
}
#[tokio::test]
#[ignore]
async fn cli_logs_follow_prints_new_lines() {
    let mut c = clean().await;
    import("base.ovpn", "base");
    let history: LogsResult = c
        .call(
            "logs",
            &LogsParams {
                after: None,
                limit: 1000,
            },
        )
        .await
        .unwrap();
    assert!(
        !history.entries.is_empty(),
        "disconnect audit must provide the history barrier"
    );
    let mut child = spawn_as("alice", &["logs", "-f"]);
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    // Drain history completely. Receiving an old line cannot satisfy this test.
    for _ in &history.entries {
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    success(
        &["connect", "base", "--username", "alice", "--password-stdin"],
        b"alice-secret\n",
    );
    assert!(!rx.recv_timeout(Duration::from_secs(5)).unwrap().is_empty());
    let _ = sh(&format!("pkill -TERM -P {}", child.id()));
    let _ = child.wait();
    reader.join().unwrap();
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn cli_ambiguous_name_lists_candidates() {
    clean().await;
    let a = import("base.ovpn", "duplicate");
    let b = import("split.ovpn", "duplicate");
    let out = cli(&["connect", "duplicate"], &[]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(&a));
    assert!(err.contains(&b));
}
#[tokio::test]
#[ignore]
async fn cli_profile_files_are_private() {
    clean().await;
    import("base.ovpn", "base");
    import("wg-split.conf", "office");
    checked(
        "python3 - <<'PY'\nfrom pathlib import Path\nroot=Path('/home/alice/.config/galleria')\nfor folder in ['profiles','secrets']:\n d=root/folder\n assert d.stat().st_mode & 0o777 == 0o700\n files=list(d.glob('*.toml')); assert len(files)==2\n for p in files: assert p.stat().st_mode & 0o777 == 0o600\nkey=Path('/lab/lab/out/base.ovpn').read_text().split('<key>')[1].split('</key>')[0]\nneedle=next(l for l in key.splitlines() if len(l)>40 and not l.startswith('-'))\nfor p in (root/'profiles').glob('*.toml'): assert needle not in p.read_text()\nPY",
    );
}
#[tokio::test]
#[ignore]
async fn daemon_never_opened_user_files() {
    let mut c = clean().await;
    import("base.ovpn", "base");
    success(
        &["connect", "base", "--username", "alice", "--password-stdin"],
        b"alice-secret\n",
    );
    assert_eq!(
        checked("systemctl show galleriad -p ProtectHome --value"),
        "yes"
    );
    let pid = checked("systemctl show galleriad -p MainPID --value");
    let files = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap()
        .map(|f| std::fs::read_link(f.unwrap().path()).unwrap())
        .collect::<Vec<_>>();
    assert!(files.iter().all(|p| !p.starts_with("/home")));
    disconnect(&mut c).await;
}
#[tokio::test]
#[ignore]
async fn cli_interactive_password_is_requested_once_without_echo() {
    use std::io::Read;
    let mut c = clean().await;
    import("base.ovpn", "base");
    let mut child = Command::new("runuser")
        .args([
            "-u",
            "alice",
            "--",
            "script",
            "-q",
            "-c",
            "/usr/local/bin/galleria connect base --username alice",
            "/dev/null",
        ])
        .env("LC_ALL", "C")
        .env("XDG_CONFIG_HOME", "/home/alice/.config")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut output = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut b = [0u8; 1];
        while output.read(&mut b).unwrap_or(0) > 0 {
            if tx.send(b[0]).is_err() {
                break;
            }
        }
    });
    let until = Instant::now() + Duration::from_secs(10);
    let mut bytes = Vec::new();
    while !String::from_utf8_lossy(&bytes).contains("Password: ") && Instant::now() < until {
        if let Ok(b) = rx.recv_timeout(Duration::from_millis(50)) {
            bytes.push(b);
        }
    }
    let prompted = String::from_utf8_lossy(&bytes).contains("Password: ");
    if prompted {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"alice-secret\n")
            .unwrap();
    }
    let until = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() && Instant::now() < until {
        if let Ok(b) = rx.recv_timeout(Duration::from_millis(20)) {
            bytes.push(b);
        }
    }
    let exited = child.try_wait().unwrap().is_some();
    let _ = sh("pkill -TERM -u alice -x galleria");
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    bytes.extend(rx.try_iter());
    let text = String::from_utf8_lossy(&bytes);
    assert!(prompted, "password prompt absent");
    assert!(
        exited,
        "interactive connect stalled; prompt count {}",
        text.matches("Password: ").count()
    );
    assert_eq!(text.matches("Password: ").count(), 1);
    assert!(text.contains("connected"));
    assert!(!text.contains("alice-secret"));
    disconnect(&mut c).await;
}

#[tokio::test]
#[ignore]
async fn cli_imports_binary_pkcs12_reference_and_connects() {
    let mut c = clean().await;
    let dir = "/home/alice/incoming";
    std::fs::copy("/lab/lab/out/client.p12", format!("{dir}/client.p12")).unwrap();
    let mut items = galleria_ovpn::parse(&lab_file("base.ovpn")).unwrap();
    items.retain(|item|!matches!(item,galleria_ovpn::Item::Block {tag,..} if ["ca","cert","key"].contains(&tag.as_str())));
    items.push(galleria_ovpn::Item::Directive {
        line: 1,
        name: "pkcs12".into(),
        args: vec!["client.p12".into()],
    });
    std::fs::write(format!("{dir}/pkcs12.ovpn"), galleria_ovpn::render(&items)).unwrap();
    checked(
        "chown alice:alice /home/alice/incoming/client.p12 /home/alice/incoming/pkcs12.ovpn; chmod 600 /home/alice/incoming/client.p12 /home/alice/incoming/pkcs12.ovpn",
    );
    success(
        &[
            "import",
            "/home/alice/incoming/pkcs12.ovpn",
            "--name",
            "p12",
        ],
        &[],
    );
    checked(
        "python3 - <<'PY'\nfrom pathlib import Path\nimport tomllib,base64\nroot=Path('/home/alice/.config/galleria')\np=next((root/'profiles').glob('*.toml'))\nprofile=tomllib.loads(p.read_text())\narchive=tomllib.loads((root/'secrets'/(profile['id']+'.toml')).read_text())['openvpn.block.pkcs12']\nassert base64.b64decode(archive)==Path('/home/alice/incoming/client.p12').read_bytes()\nassert archive.splitlines()[0] not in p.read_text()\nassert '<pkcs12>' not in profile['openvpn']['config']\nPY",
    );
    success(
        &["connect", "p12", "--username", "alice", "--password-stdin"],
        b"alice-secret\n",
    );
    assert_eq!(success(&["status"], &[]).trim(), "connected");
    disconnect(&mut c).await;
}
