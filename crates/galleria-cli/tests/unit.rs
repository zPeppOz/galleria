#![allow(clippy::unwrap_used, clippy::expect_used)]
use clap::Parser;
use galleria_cli::{cli::*, commands::*, messages::*, prompt::*};
use galleria_ipc::*;
use rstest::rstest;
#[test]
fn parses_every_subcommand() {
    for args in [
        vec!["import", "base.ovpn", "--name", "Office"],
        vec!["list"],
        vec!["delete", "Office"],
        vec![
            "connect",
            "Office",
            "--replace",
            "--username",
            "alice",
            "--password-stdin",
            "--otp-stdin",
            "--no-wait",
        ],
        vec!["disconnect"],
        vec!["status", "--json"],
        vec!["logs", "-f"],
        vec!["version"],
    ] {
        assert!(Cli::try_parse_from(std::iter::once("galleria").chain(args)).is_ok());
    }
}
#[rstest]
#[case("ok", 0)]
#[case("already_active", 1)]
#[case("invalid_config", 1)]
#[case("auth_failed", 2)]
#[case("needs_input", 3)]
#[case("server_unreachable", 4)]
#[case("permission_denied", 5)]
#[case("daemon_unreachable", 5)]
#[case("incompatible_protocol", 5)]
fn exit_code_follows_the_table(#[case] code: &str, #[case] wanted: u8) {
    assert_eq!(exit_code(code), wanted);
}
#[test]
fn every_error_code_has_both_languages() {
    for code in [
        "hello_required",
        "incompatible_protocol",
        "message_too_large",
        "too_many_clients",
        "already_active",
        "not_active",
        "invalid_config",
        "profile_rejected",
        "backend_unavailable",
        "input_not_expected",
        "internal",
        "auth_failed",
        "input_timeout",
        "server_unreachable",
        "tunnel_setup_failed",
        "net_setup_failed",
        "dns_unavailable",
        "unsupported_challenge",
        "backend_exited",
    ] {
        let en = text(code, Lang::En);
        let it = text(code, Lang::It);
        assert!(!en.is_empty());
        assert!(!it.is_empty());
        assert_ne!(en, it, "{code}");
    }
}
#[test]
fn permission_denied_message_names_the_group() {
    for lang in [Lang::En, Lang::It] {
        assert!(text("permission_denied", lang).contains("galleria"));
        assert!(text("permission_denied", lang).contains("usermod -aG"));
    }
}
#[test]
fn incompatible_protocol_message_is_actionable() {
    let message = incompatible_message(
        &serde_json::json!({"daemon_version":"9.8.7","daemon_min":2,"daemon_max":3}),
        Lang::En,
    );
    assert!(message.contains("9.8.7"));
    assert!(message.contains(env!("CARGO_PKG_VERSION")));
    assert!(message.contains("2"));
    assert!(message.contains("3"));
    assert!(message.contains("1"));
}
#[test]
fn non_tty_needs_input_exits_3() {
    for kind in [
        InputKind::Credentials,
        InputKind::Otp,
        InputKind::KeyPassphrase,
    ] {
        let request = InputRequest {
            request: 1,
            kind,
            prompt: "Input".into(),
            echo: false,
            expires_in_secs: 120,
        };
        assert_eq!(require_terminal(&request, false).unwrap_err().code, 3);
    }
}
#[rstest]
#[case("--otp", "123456")]
#[case("--password", "pw")]
#[case("--key-passphrase", "pw")]
fn secret_valued_options_do_not_exist(#[case] flag: &str, #[case] value: &str) {
    assert!(Cli::try_parse_from(["galleria", "connect", "x", flag, value]).is_err());
}
#[test]
fn stdin_secrets_are_read_in_order() {
    let values = stdin_secrets(&mut &b"pw\r\n123456\n"[..], true, true).unwrap();
    assert_eq!(values.password.unwrap().expose(), "pw");
    assert_eq!(values.otp.unwrap().expose(), "123456");
    let values = stdin_secrets(&mut &b"123456\n"[..], false, true).unwrap();
    assert!(values.password.is_none());
    assert_eq!(values.otp.unwrap().expose(), "123456");
}
#[test]
fn prompt_never_echoes() {
    use nix::{
        pty::openpty,
        sys::termios::{LocalFlags, tcgetattr},
    };
    use std::{
        fs::File,
        io::Write,
        time::{Duration, Instant},
    };
    let pair = openpty(None, None).unwrap();
    let mut master = File::from(pair.master);
    let slave = File::from(pair.slave);
    let observer = slave.try_clone().unwrap();
    let before = tcgetattr(&slave).unwrap();
    let writer = std::thread::spawn(move || {
        let start = Instant::now();
        while tcgetattr(&observer)
            .unwrap()
            .local_flags
            .contains(LocalFlags::ECHO)
        {
            assert!(start.elapsed() < Duration::from_secs(2));
            std::thread::yield_now();
        }
        master.write_all(b"private-input\n").unwrap();
        master
    });
    assert_eq!(read_hidden_from(&slave).unwrap().expose(), "private-input");
    let _master = writer.join().unwrap();
    assert_eq!(tcgetattr(&slave).unwrap().local_flags, before.local_flags);
}
#[test]
fn status_json_is_the_ipc_status() {
    let status = Status::default();
    let output = status_json(&status).unwrap();
    assert_eq!(serde_json::from_str::<Status>(&output).unwrap(), status);
}

#[test]
fn echo_is_disabled_before_the_prompt_becomes_observable() {
    use nix::{
        pty::openpty,
        sys::termios::{LocalFlags, tcgetattr},
    };
    use std::{
        fs::File,
        io::{self, Write},
    };
    struct Observer {
        tty: File,
        master: File,
        sent: bool,
    }
    impl Write for Observer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            assert!(
                !tcgetattr(&self.tty)
                    .unwrap()
                    .local_flags
                    .contains(LocalFlags::ECHO),
                "visible prompt while echo enabled"
            );
            if !self.sent {
                self.master.write_all(b"synthetic-secret\n")?;
                self.sent = true;
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            assert!(
                !tcgetattr(&self.tty)
                    .unwrap()
                    .local_flags
                    .contains(LocalFlags::ECHO)
            );
            Ok(())
        }
    }
    let pair = openpty(None, None).unwrap();
    let tty = File::from(pair.slave);
    let before = tcgetattr(&tty).unwrap();
    let mut observer = Observer {
        tty: tty.try_clone().unwrap(),
        master: File::from(pair.master),
        sent: false,
    };
    let secret = prompt_hidden_from(&tty, "Password", &mut observer).unwrap();
    assert_eq!(secret.expose(), "synthetic-secret");
    assert!(observer.sent);
    assert_eq!(tcgetattr(&tty).unwrap().local_flags, before.local_flags);
}
