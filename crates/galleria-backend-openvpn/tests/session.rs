#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_backend::{net::Ipv6Block, *};
use galleria_backend_openvpn::{config::Redirect, hook::*, hookenv::*, session::*};
use galleria_ipc::*;
use rstest::rstest;
use std::{collections::BTreeMap, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf},
    sync::{mpsc, oneshot},
};
struct Harness {
    read: BufReader<ReadHalf<DuplexStream>>,
    output: DuplexStream,
    write: WriteHalf<DuplexStream>,
    commands: mpsc::Sender<BackendCommand>,
    hooks: mpsc::Sender<HookCall>,
    events: mpsc::Receiver<BackendEvent>,
    task: tokio::task::JoinHandle<ExitReason>,
}
impl Harness {
    fn new(values: InputValues) -> Self {
        Self::with_ignore(values, false)
    }
    fn with_ignore(values: InputValues, ignore: bool) -> Self {
        let (a, b) = tokio::io::duplex(65536);
        let (stdout, output) = tokio::io::duplex(65536);
        let (read, write) = tokio::io::split(b);
        let (commands, rx) = mpsc::channel(16);
        let (hooks, hr) = mpsc::channel(16);
        let (events, er) = mpsc::channel(128);
        let ctx = SessionContext {
            session: 1,
            session_dir: Default::default(),
            events,
        };
        let task = tokio::spawn(async move {
            drive_with_output(
                a,
                Credentials::new(
                    values,
                    ProfileFacts {
                        redirect: Redirect::default(),
                        ipv6_routes: vec![],
                    },
                    true,
                )
                .ignore_pushed_routes(ignore),
                &ctx,
                rx,
                hr,
                stdout,
            )
            .await
        });
        Self {
            read: BufReader::new(read),
            output,
            write,
            commands,
            hooks,
            events: er,
            task,
        }
    }
    async fn line(&mut self, s: &str) {
        self.write
            .write_all(format!("{s}\n").as_bytes())
            .await
            .unwrap();
    }
    async fn command(&mut self) -> String {
        let mut s = String::new();
        tokio::time::timeout(Duration::from_secs(2), self.read.read_line(&mut s))
            .await
            .unwrap()
            .unwrap();
        s.trim_end().into()
    }
    async fn event(&mut self) -> BackendEvent {
        tokio::time::timeout(Duration::from_secs(2), self.events.recv())
            .await
            .unwrap()
            .unwrap()
    }
    async fn setup(&mut self) {
        self.line(">INFO:test").await;
        self.line(">HOLD:waiting").await;
        for s in [
            "version 3",
            "state on",
            "bytecount 1",
            "log on",
            "hold release",
        ] {
            assert_eq!(self.command().await, s);
            self.line("SUCCESS: ok").await;
        }
    }
    async fn hook(&self, kind: &str) -> oneshot::Receiver<bool> {
        self.hook_with(kind, &[]).await
    }
    async fn hook_with(&self, kind: &str, extra: &[(&str, &str)]) -> oneshot::Receiver<bool> {
        let mut env: BTreeMap<String, String> = [("dev", "glr-ovpn0"), ("script_type", kind)]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        for (k, v) in extra {
            env.insert((*k).into(), (*v).into());
        }
        let (reply, rx) = oneshot::channel();
        self.hooks.send(HookCall { env, reply }).await.unwrap();
        rx
    }
    async fn netup(&mut self) -> galleria_backend::net::NetConfig {
        let BackendEvent::NetUp { config, reply } = self.event().await else {
            panic!("expected NetUp")
        };
        reply.send(Ok(())).unwrap();
        config
    }
    async fn connect(&mut self) {
        let verdict = self.hook("up").await;
        self.netup().await;
        assert!(verdict.await.unwrap());
        self.line(">STATE:0,CONNECTED,SUCCESS,10.8.0.2,172.31.0.10,1194,,")
            .await;
        assert!(matches!(self.event().await, BackendEvent::Up(_)));
    }
    async fn finish(mut self) {
        self.commands.send(BackendCommand::Stop).await.unwrap();
        assert_eq!(self.command().await, "signal SIGTERM");
        self.line(">STATE:0,EXITING,SIGTERM,,,,,").await;
        assert!(matches!(self.task.await.unwrap(), ExitReason::Stopped));
    }
}
fn values() -> InputValues {
    InputValues {
        username: Some("alice".into()),
        password: Some(Secret::new("pw")),
        ..Default::default()
    }
}
#[tokio::test]
async fn releases_hold_after_setup_commands() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.finish().await;
}
#[tokio::test]
async fn hook_up_is_answered_before_connected() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">STATE:0,ASSIGN_IP,,10.8.0.2,,,,").await;
    assert!(matches!(h.event().await, BackendEvent::Progress(_)));
    let v = h.hook("up").await;
    h.netup().await;
    assert!(v.await.unwrap());
    assert!(h.events.try_recv().is_err());
    h.finish().await;
}
#[tokio::test]
async fn up_is_sent_at_connected_after_netup_is_accepted() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.connect().await;
    h.finish().await;
}
#[tokio::test]
async fn connected_without_accepted_netup_sends_no_up() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">STATE:0,CONNECTED,SUCCESS,10.8.0.2,172.31.0.10,1194,,")
        .await;
    h.line(">LOG:0,I,barrier").await;
    assert!(matches!(h.event().await, BackendEvent::Log(_)));
    assert!(h.events.try_recv().is_err());
    h.finish().await;
}
#[tokio::test]
async fn netup_refusal_makes_hook_fail() {
    let mut h = Harness::new(values());
    h.setup().await;
    let v = h.hook("up").await;
    let BackendEvent::NetUp { reply, .. } = h.event().await else {
        panic!()
    };
    reply.send(Err("refused".into())).unwrap();
    assert!(!v.await.unwrap());
    assert!(matches!(
        h.task.await.unwrap(),
        ExitReason::Fatal(SessionError {
            code: SessionErrorCode::NetSetupFailed,
            ..
        })
    ));
}
#[tokio::test]
async fn management_is_read_while_netup_is_pending() {
    let mut h = Harness::new(values());
    h.setup().await;
    let v = h.hook("up").await;
    let BackendEvent::NetUp { reply, .. } = h.event().await else {
        panic!()
    };
    h.line(">LOG:0,I,still reading").await;
    assert!(matches!(h.event().await,BackendEvent::Log(l) if l.message=="still reading"));
    reply.send(Ok(())).unwrap();
    assert!(v.await.unwrap());
    h.finish().await;
}
#[tokio::test]
async fn internal_restart_emits_restarting_netdown_and_new_netup() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.connect().await;
    h.line(">STATE:0,RECONNECTING,ping-restart,,,,,").await;
    assert!(matches!(h.event().await,BackendEvent::Restarting(s) if s=="ping-restart"));
    let v = h.hook("down").await;
    let BackendEvent::NetDown { reply } = h.event().await else {
        panic!()
    };
    reply.send(()).unwrap();
    assert!(v.await.unwrap());
    for s in ["WAIT", "AUTH"] {
        h.line(&format!(">STATE:0,{s},,,,,,")).await;
        assert!(matches!(h.event().await, BackendEvent::Progress(_)));
    }
    h.line(">LOG:0,I,PUSH_REPLY,dhcp-option DNS 10.8.0.9").await;
    assert!(matches!(h.event().await, BackendEvent::Log(_)));
    let v = h
        .hook_with("up", &[("foreign_option_1", "dhcp-option DNS 10.8.0.9")])
        .await;
    assert_eq!(h.netup().await.dns_servers[0].to_string(), "10.8.0.9");
    assert!(v.await.unwrap());
    h.line(">STATE:0,CONNECTED,SUCCESS,10.8.0.2,172.31.0.10,1194,,")
        .await;
    assert!(matches!(h.event().await, BackendEvent::Up(_)));
    h.finish().await;
}
#[tokio::test]
async fn restart_forgets_the_previous_push_reply() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">LOG:0,I,PUSH_REPLY,redirect-gateway def1").await;
    let _ = h.event().await;
    h.connect().await;
    h.line(">STATE:0,RECONNECTING,ping-restart,,,,,").await;
    let _ = h.event().await;
    let v = h.hook("up").await;
    assert_eq!(h.netup().await.ipv6_block, Ipv6Block::None);
    assert!(v.await.unwrap());
    h.finish().await;
}
#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[tokio::test]
async fn restart_events_are_tolerated_in_any_order(#[case] order: u8) {
    let mut h = Harness::new(values());
    h.setup().await;
    h.connect().await;
    if order == 0 {
        let v = h.hook("down").await;
        let BackendEvent::NetDown { reply } = h.event().await else {
            panic!()
        };
        reply.send(()).unwrap();
        assert!(v.await.unwrap());
    }
    h.line(">STATE:0,RECONNECTING,ping-restart,,,,,").await;
    assert!(matches!(h.event().await, BackendEvent::Restarting(_)));
    if order == 1 {
        let v = h.hook("down").await;
        let BackendEvent::NetDown { reply } = h.event().await else {
            panic!()
        };
        reply.send(()).unwrap();
        assert!(v.await.unwrap());
    }
    h.connect().await;
    h.finish().await;
}
#[tokio::test]
async fn hook_down_is_answered_while_stopping() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.connect().await;
    h.commands.send(BackendCommand::Stop).await.unwrap();
    assert_eq!(h.command().await, "signal SIGTERM");
    let v = h.hook("down").await;
    let BackendEvent::NetDown { reply } = h.event().await else {
        panic!()
    };
    reply.send(()).unwrap();
    assert!(v.await.unwrap());
    h.line(">STATE:0,EXITING,SIGTERM,,,,,").await;
    assert!(matches!(h.task.await.unwrap(), ExitReason::Stopped));
}
#[tokio::test]
async fn answers_auth_with_credentials() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">PASSWORD:Need 'Auth' username/password").await;
    assert_eq!(h.command().await, "username \"Auth\" \"alice\"");
    h.line("SUCCESS: ok").await;
    assert_eq!(h.command().await, "password \"Auth\" \"pw\"");
    h.line("SUCCESS: ok").await;
    h.finish().await;
}
#[tokio::test]
async fn missing_otp_asks_for_input() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">PASSWORD:Need 'Auth' username/password SC:1,Enter OTP")
        .await;
    assert!(
        matches!(h.event().await,BackendEvent::InputRequired(InputRequest{kind:InputKind::Otp,prompt,..}) if prompt=="Enter OTP")
    );
    h.commands
        .send(BackendCommand::ProvideInput(InputValues {
            otp: Some(Secret::new("123456")),
            ..Default::default()
        }))
        .await
        .unwrap();
    assert_eq!(h.command().await, "username \"Auth\" \"alice\"");
    h.line("SUCCESS: ok").await;
    assert_eq!(
        h.command().await,
        "password \"Auth\" \"SCRV1:cHc=:MTIzNDU2\""
    );
    h.finish().await;
}
#[tokio::test]
async fn verification_failed_exits_auth_failed() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">PASSWORD:Verification Failed: 'Auth'").await;
    assert!(matches!(h.task.await.unwrap(), ExitReason::AuthFailed(_)));
}
#[tokio::test]
async fn exiting_auth_failure_exits_auth_failed() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">STATE:0,EXITING,auth-failure,,,,,").await;
    assert!(matches!(h.task.await.unwrap(), ExitReason::AuthFailed(_)));
}
#[tokio::test]
async fn dynamic_challenge_exits_unsupported() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">PASSWORD:Need 'Auth' CRV1:x").await;
    assert!(matches!(
        h.task.await.unwrap(),
        ExitReason::Fatal(SessionError {
            code: SessionErrorCode::UnsupportedChallenge,
            ..
        })
    ));
}
#[tokio::test]
async fn socket_close_exits_transport() {
    let h = Harness::new(values());
    drop(h.write);
    drop(h.read);
    assert!(matches!(h.task.await.unwrap(), ExitReason::Transport(_)));
}
#[tokio::test]
async fn stop_sends_sigterm_signal() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.finish().await;
}
#[tokio::test]
async fn push_reply_in_log_is_remembered_for_netup() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">LOG:0,I,PUSH_REPLY,redirect-gateway def1").await;
    let _ = h.event().await;
    let v = h.hook("up").await;
    assert_eq!(h.netup().await.ipv6_block, Ipv6Block::Unreachable);
    assert!(v.await.unwrap());
    h.finish().await;
}
#[test]
fn hook_forwards_only_allowed_env() {
    let e = [
        ("password".into(), "x".into()),
        ("foreign_option_1".into(), "dhcp-option DNS 10.8.0.1".into()),
        ("device_password".into(), "x".into()),
    ]
    .into_iter()
    .collect();
    let e = filter_env(e);
    assert_eq!(e.len(), 1);
    assert!(e.contains_key("foreign_option_1"));
}
#[test]
fn hook_exit_code_follows_verdict() {
    assert_eq!(exit_code("up", Some(true)), 0);
    assert_eq!(exit_code("up", Some(false)), 1);
    assert_eq!(exit_code("up", None), 1);
    assert_eq!(exit_code("down", Some(false)), 0);
    assert_eq!(exit_code("down", None), 0);
}
#[tokio::test]
async fn queued_push_is_read_before_hook() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.line(">LOG:0,I,PUSH_REPLY,redirect-gateway def1").await;
    let v = h.hook("up").await;
    assert!(matches!(h.event().await, BackendEvent::Log(_)));
    assert_eq!(h.netup().await.ipv6_block, Ipv6Block::Unreachable);
    assert!(v.await.unwrap());
    h.finish().await;
}

#[tokio::test]
async fn ignored_push_routes_do_not_affect_netup() {
    let mut h = Harness::with_ignore(values(), true);
    h.setup().await;
    h.line(">LOG:0,I,PUSH_REPLY,redirect-gateway def1,route 192.0.2.0 255.255.255.0")
        .await;
    let _ = h.event().await;
    let v = h.hook("up").await;
    let cfg = h.netup().await;
    assert_eq!(cfg.ipv6_block, Ipv6Block::None);
    assert!(cfg.routes.is_empty());
    assert!(v.await.unwrap());
    h.finish().await;
}

#[tokio::test]
async fn stdout_push_is_read_before_hook_when_management_log_is_delayed() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.output
        .write_all(
            b"2026-11-12 PUSH: Received control message: 'PUSH_REPLY,redirect-gateway def1'\n",
        )
        .await
        .unwrap();
    let v = h.hook("up").await;
    assert!(matches!(h.event().await, BackendEvent::Log(_)));
    assert_eq!(h.netup().await.ipv6_block, Ipv6Block::Unreachable);
    assert!(v.await.unwrap());
    h.finish().await;
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn continued_push_retains_redirect_until_final_part(#[case] stdout: bool) {
    let mut h = Harness::new(InputValues::default());
    h.setup().await;
    for text in [
        "PUSH_REPLY,redirect-gateway def1,push-continuation 2",
        "PUSH_REPLY,route 192.0.2.0 255.255.255.0,push-continuation 2",
        "PUSH_REPLY,dhcp-option DNS 10.8.0.1,push-continuation 1",
    ] {
        if stdout {
            h.output
                .write_all(format!("{text}\n").as_bytes())
                .await
                .unwrap();
        } else {
            h.line(&format!(">LOG:0,I,{text}")).await;
        }
        assert!(matches!(h.event().await, BackendEvent::Log(_)));
    }
    let verdict = h.hook("up").await;
    let config = h.netup().await;
    assert_eq!(config.ipv6_block, Ipv6Block::Unreachable);
    assert!(config.routes.contains(&"192.0.2.0/24".parse().unwrap()));
    assert!(verdict.await.unwrap());
    h.line(">STATE:0,RECONNECTING,restart,,,,,").await;
    h.line(">STATE:0,WAIT,,,,,,").await;
    assert!(matches!(h.event().await, BackendEvent::Progress(_)));
    h.line(">LOG:0,I,PUSH_REPLY,dhcp-option DNS 10.9.0.1").await;
    assert!(matches!(h.event().await, BackendEvent::Log(_)));
    let verdict = h.hook("up").await;
    assert_eq!(h.netup().await.ipv6_block, Ipv6Block::None);
    assert!(verdict.await.unwrap());
    h.commands.send(BackendCommand::Stop).await.unwrap();
    h.task.abort();
}

#[tokio::test]
async fn internal_restart_releases_the_new_management_hold() {
    let mut h = Harness::new(values());
    h.setup().await;
    h.connect().await;
    h.line(">STATE:0,RECONNECTING,SIGUSR1,,,,,").await;
    assert!(matches!(h.event().await, BackendEvent::Restarting(_)));
    h.line(">HOLD:Waiting for hold release:1").await;
    assert_eq!(h.command().await, "hold release");
    h.line("SUCCESS: hold release succeeded").await;
    h.connect().await;
    h.finish().await;
}
