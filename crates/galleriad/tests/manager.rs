#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use galleria_backend::{Backend, BackendCommand, BackendEvent, ExitReason, fake::*};
use galleria_ipc::*;
use galleriad::{machine::RetryPolicy, manager::*};
use std::sync::{Arc, Mutex};
use std::{sync::atomic::Ordering, time::Duration};
use support::*;
use tokio::sync::oneshot;
#[tokio::test(start_paused = true)]
async fn replace_waits_for_teardown_before_spawning() {
    let h = Harness::new(vec![online(), online()]);
    h.handle.connect(params(false), 1000).await.unwrap();
    h.state(State::Connected).await;
    h.handle.connect(params(true), 1000).await.unwrap();
    h.state(State::Connected).await;
    let log = h.net.log.lock().unwrap().clone();
    let start = log.iter().position(|s| s == "teardown").unwrap();
    let stop = log.iter().position(|s| s == "stop").unwrap();
    let exit = log.iter().position(|s| s == "exit").unwrap();
    let spawn = log.iter().position(|s| s == "spawn(2)").unwrap();
    assert!(start < stop && stop < exit && exit < spawn, "{log:?}");
    h.handle.disconnect(1000).await;
}
#[tokio::test(start_paused = true)]
async fn net_teardown_precedes_backend_stop() {
    let h = Harness::new(vec![online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    h.handle.disconnect(1).await;
    let log = h.net.log.lock().unwrap();
    let teardown = log
        .iter()
        .position(|s| s == "teardown")
        .expect("teardown missing");
    let stop = log.iter().position(|s| s == "stop").expect("Stop missing");
    assert!(teardown < stop, "{log:?}");
}
#[tokio::test(start_paused = true)]
async fn connect_while_active_is_already_active() {
    let h = Harness::new(vec![online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    let e = h.handle.connect(params(false), 2).await.unwrap_err();
    assert_eq!(e.data.as_ref().unwrap()["code"], "already_active");
    assert_eq!(e.data.unwrap()["profile"]["name"], "Test VPN");
    h.handle.disconnect(1).await;
}
#[tokio::test(start_paused = true)]
async fn disconnect_is_idempotent() {
    let h = Harness::new(vec![online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    assert_eq!(h.handle.disconnect(1).await.state, State::Disconnected);
    assert_eq!(h.handle.disconnect(1).await.state, State::Disconnected);
}
#[tokio::test(start_paused = true)]
async fn netup_refusal_sets_net_setup_failed() {
    let h = Harness::new(vec![online()]);
    h.net.refuse.store(true, Ordering::SeqCst);
    h.handle.connect(params(false), 1).await.unwrap();
    let s = h.state(State::Error).await;
    assert_eq!(s.error.unwrap().code, SessionErrorCode::NetSetupFailed);
    assert_eq!(h.live.load(Ordering::SeqCst), 0);
}
#[tokio::test(start_paused = true)]
async fn connected_only_after_net_applied() {
    let mut script = vec![up()];
    script.extend(online());
    let h = Harness::new(vec![script]);
    let mut events = h.handle.events();
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    let mut found = false;
    while let Ok(e) = events.try_recv() {
        if let Event::State(s) = e
            && s.state == State::Connected
        {
            assert!(
                s.net.is_some(),
                "connected published before NetUp was accepted"
            );
            found = true;
        }
    }
    assert!(found);
    h.handle.disconnect(1).await;
}
#[tokio::test(start_paused = true)]
async fn needs_input_times_out_after_120s() {
    let h = Harness::new(vec![vec![FakeStep::Emit(FakeEvent::InputRequired(
        InputRequest {
            request: 7,
            kind: InputKind::Otp,
            prompt: "Enter OTP".into(),
            echo: false,
            expires_in_secs: 120,
        },
    ))]]);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::NeedsInput).await;
    tokio::time::advance(Duration::from_secs(120)).await;
    assert_eq!(
        h.state(State::Error).await.error.unwrap().code,
        SessionErrorCode::InputTimeout
    );
}
#[tokio::test(start_paused = true)]
async fn transport_exits_retry_four_times_then_give_up() {
    let scripts = (0..4)
        .map(|_| {
            vec![FakeStep::Emit(FakeEvent::Exited(ExitReason::Transport(
                "test".into(),
            )))]
        })
        .collect();
    let h = Harness::new(scripts);
    h.handle.connect(params(false), 1).await.unwrap();
    for (n, delay) in [1, 2, 4].into_iter().enumerate() {
        for _ in 0..100 {
            tokio::task::yield_now().await;
        }
        assert_eq!(h.spawns.load(Ordering::SeqCst), n + 1);
        tokio::time::advance(Duration::from_secs(delay)).await;
    }
    assert_eq!(
        h.state(State::Error).await.error.unwrap().code,
        SessionErrorCode::ServerUnreachable
    );
    assert_eq!(h.spawns.load(Ordering::SeqCst), 4);
}
#[tokio::test(start_paused = true)]
async fn netup_is_answered_without_waiting_for_up() {
    let h = Harness::new(vec![vec![
        FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.1"))),
        FakeStep::Emit(FakeEvent::Progress("netup answered".into())),
    ]]);
    h.handle.connect(params(false), 1).await.unwrap();
    for _ in 0..100 {
        let s = h.handle.status().await;
        if s.detail.as_deref() == Some("netup answered") {
            assert_eq!(s.state, State::Connecting);
            h.handle.disconnect(1).await;
            return;
        }
    }
    panic!("NetUp deadlocked");
}
#[tokio::test(start_paused = true)]
async fn up_before_accepted_netup_does_not_connect() {
    let h = Harness::new(vec![vec![up()]]);
    h.handle.connect(params(false), 1).await.unwrap();
    for _ in 0..50 {
        assert_ne!(h.handle.status().await.state, State::Connected);
    }
    h.handle.disconnect(1).await;
}
#[tokio::test(start_paused = true)]
async fn internal_restart_reapplies_network() {
    let mut script = online();
    script.extend([
        FakeStep::Emit(FakeEvent::Restarting("restart".into())),
        FakeStep::Emit(FakeEvent::NetDown),
        FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.9"))),
        up(),
    ]);
    let h = Harness::new(vec![script]);
    let mut events = h.handle.events();
    h.handle.connect(params(false), 1).await.unwrap();
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    let s = h.state(State::Connected).await;
    assert_eq!(
        s.net.unwrap().dns_servers[0],
        "10.0.0.9".parse::<std::net::IpAddr>().unwrap()
    );
    let states = std::iter::from_fn(|| events.try_recv().ok())
        .filter_map(|e| {
            if let Event::State(s) = e {
                Some(s.state)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let phases: Vec<_> = states
        .into_iter()
        .filter(|s| matches!(s, State::Connected | State::Reconnecting))
        .fold(Vec::new(), |mut phases, s| {
            if phases.last() != Some(&s) {
                phases.push(s);
            }
            phases
        });
    assert_eq!(
        phases,
        [State::Connected, State::Reconnecting, State::Connected]
    );
    let log = h.net.log.lock().unwrap().clone();
    let a = log.iter().position(|s| s == "apply(10.0.0.1)").unwrap();
    let down = log.iter().position(|s| s == "teardown").unwrap();
    let b = log.iter().position(|s| s == "apply(10.0.0.9)").unwrap();
    assert!(a < down && down < b, "{log:?}");
    assert_eq!(h.spawns.load(Ordering::SeqCst), 1);
    h.handle.disconnect(1).await;
}
#[tokio::test(start_paused = true)]
async fn second_netup_without_netdown_replaces_the_first() {
    let mut script = online();
    script.extend([FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.9"))), up()]);
    let h = Harness::new(vec![script]);
    h.handle.connect(params(false), 1).await.unwrap();
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    let log = h.net.log.lock().unwrap().clone();
    let a = log.iter().position(|s| s == "apply(10.0.0.1)").unwrap();
    let down = log.iter().position(|s| s == "teardown").unwrap();
    let b = log.iter().position(|s| s == "apply(10.0.0.9)").unwrap();
    assert!(a < down && down < b, "{log:?}");
    h.handle.disconnect(1).await;
}
#[tokio::test(start_paused = true)]
async fn netdown_is_answered_during_disconnect() {
    let (_tmp, handle, stopped, answered, finish) = stop_probe(false);
    handle.connect(params(false), 1).await.unwrap();
    wait_state(&handle, State::Connected).await;
    let disconnect = tokio::spawn({
        let handle = handle.clone();
        async move { handle.disconnect(1).await }
    });
    stopped.await.unwrap();
    assert_eq!(handle.status().await.state, State::Disconnecting);
    assert!(
        tokio::time::timeout(Duration::from_secs(1), answered)
            .await
            .unwrap()
            .unwrap(),
        "NetDown reply was dropped"
    );
    assert!(!disconnect.is_finished(), "backend must still be alive");
    finish.send(()).unwrap();
    assert_eq!(disconnect.await.unwrap().state, State::Disconnected);
}
#[tokio::test(start_paused = true)]
async fn each_spawn_gets_a_fresh_session_dir() {
    let h = Harness::new(vec![online(), online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    h.handle.connect(params(true), 1).await.unwrap();
    h.state(State::Connected).await;
    let dirs = h.dirs.lock().unwrap().clone();
    assert_eq!(dirs.len(), 2);
    assert_ne!(dirs[0], dirs[1]);
    assert!(!dirs[0].exists());
    assert!(dirs[1].exists());
    h.handle.disconnect(1).await;
    assert!(!dirs[1].exists());
}
proptest::proptest! {
 #![proptest_config(proptest::test_runner::Config::with_cases(256))]
 #[test]
 fn never_two_live_backends(ops in proptest::collection::vec(0u8..4,1..60)){
  tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
   tokio::time::pause();let h=Harness::new((0..100).map(|_|vec![FakeStep::Sleep(Duration::from_secs(1)),FakeStep::Emit(FakeEvent::Exited(ExitReason::Transport("drop".into())))]).collect());
   for op in ops{match op{0=>{let _=h.handle.connect(params(false),1).await;},1=>{let _=h.handle.connect(params(true),1).await;},2=>{h.handle.disconnect(1).await;},_=>{tokio::time::advance(Duration::from_secs(1)).await;}}
    assert!(h.live.load(Ordering::SeqCst)<=1);
   }h.handle.disconnect(1).await;assert_eq!(h.live.load(Ordering::SeqCst),0);
  });
 }
}

async fn assert_cleanup_failure(h: &Harness) {
    let s = h.state(State::Error).await;
    assert_eq!(s.error.unwrap().code, SessionErrorCode::NetSetupFailed);
    assert_eq!(h.live.load(Ordering::SeqCst), 0);
    tokio::time::advance(Duration::from_secs(60)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert_eq!(h.spawns.load(Ordering::SeqCst), 1);
    assert_eq!(
        h.net
            .log
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.starts_with("apply"))
            .count(),
        1
    );
    let e = h
        .handle
        .provide_input(ProvideInputParams {
            session: 1,
            request: 1,
            values: InputValues::default(),
        })
        .await
        .unwrap_err();
    assert_eq!(e.data.unwrap()["code"], "not_active");
    // No new session may reuse residual network state until cleanup succeeds.
    assert!(h.handle.connect(params(false), 1).await.is_err());
    h.net.fail_teardown.store(false, Ordering::SeqCst);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    h.handle.disconnect(1).await;
}
#[tokio::test(start_paused = true)]
async fn failed_teardown_prevents_transport_retry() {
    let mut script = online();
    script.extend([
        FakeStep::Sleep(Duration::from_secs(1)),
        FakeStep::Emit(FakeEvent::Exited(ExitReason::Transport("drop".into()))),
    ]);
    let h = Harness::new(vec![script, online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    h.net.fail_teardown.store(true, Ordering::SeqCst);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_cleanup_failure(&h).await;
}
#[tokio::test(start_paused = true)]
async fn failed_teardown_during_restart_prevents_new_netup() {
    let mut script = online();
    script.extend([
        FakeStep::Sleep(Duration::from_secs(1)),
        FakeStep::Emit(FakeEvent::Restarting("restart".into())),
        FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.9"))),
        up(),
    ]);
    let h = Harness::new(vec![script, online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    h.net.fail_teardown.store(true, Ordering::SeqCst);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_cleanup_failure(&h).await;
}
#[tokio::test(start_paused = true)]
async fn failed_netdown_prevents_new_netup() {
    let mut script = online();
    script.extend([
        FakeStep::Sleep(Duration::from_secs(1)),
        FakeStep::Emit(FakeEvent::NetDown),
        FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.9"))),
        up(),
    ]);
    let h = Harness::new(vec![script, online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    h.net.fail_teardown.store(true, Ordering::SeqCst);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_cleanup_failure(&h).await;
}
#[tokio::test(start_paused = true)]
async fn failed_replacement_startup_releases_session_and_allows_next_connect() {
    let h = Harness::new(vec![online(), online(), online()]);
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    let root = h.tmp.path().join("s");
    std::fs::rename(&root, h.tmp.path().join("old-s")).unwrap();
    std::fs::write(&root, b"not a directory").unwrap();
    assert!(h.handle.connect(params(true), 1).await.is_err());
    assert_eq!(h.handle.status().await.state, State::Error);
    assert_eq!(h.live.load(Ordering::SeqCst), 0);
    let e = h
        .handle
        .provide_input(ProvideInputParams {
            session: 2,
            request: 1,
            values: InputValues::default(),
        })
        .await
        .unwrap_err();
    assert_eq!(e.data.unwrap()["code"], "not_active");
    std::fs::remove_file(&root).unwrap();
    h.handle.connect(params(false), 1).await.unwrap();
    h.state(State::Connected).await;
    h.handle.disconnect(1).await;
}

// The backend holds its exit after Stop, so no 15 s auto-advance can hide a lost reply.
fn stop_probe(
    input: bool,
) -> (
    tempfile::TempDir,
    ManagerHandle,
    oneshot::Receiver<()>,
    oneshot::Receiver<bool>,
    oneshot::Sender<()>,
) {
    let tmp = tempfile::tempdir().unwrap();
    let (stopped_tx, stopped) = oneshot::channel();
    let (answered_tx, answered) = oneshot::channel();
    let (finish, finish_rx) = oneshot::channel();
    let backend = TestBackend::new(move |ctx, mut commands| async move {
        if input {
            ctx.events
                .send(BackendEvent::InputRequired(InputRequest {
                    request: 7,
                    kind: InputKind::Otp,
                    prompt: "OTP".into(),
                    echo: false,
                    expires_in_secs: 120,
                }))
                .await
                .unwrap();
        } else {
            let (reply, wait) = oneshot::channel();
            ctx.events
                .send(BackendEvent::NetUp {
                    config: config("10.0.0.1"),
                    reply,
                })
                .await
                .unwrap();
            wait.await.unwrap().unwrap();
            ctx.events
                .send(BackendEvent::Up(galleria_backend::TunnelInfo {
                    local_addresses: vec![],
                    remote: None,
                }))
                .await
                .unwrap();
        }
        assert!(matches!(commands.recv().await, Some(BackendCommand::Stop)));
        stopped_tx.send(()).unwrap();
        let (reply, wait) = oneshot::channel();
        ctx.events
            .send(BackendEvent::NetDown { reply })
            .await
            .unwrap();
        let _ = answered_tx.send(wait.await.is_ok());
        finish_rx.await.unwrap();
        ctx.events
            .send(BackendEvent::Exited(ExitReason::Stopped))
            .await
            .unwrap();
    });
    let first = Mutex::new(Some(backend));
    let factory: BackendFactory = Arc::new(move |_, _| {
        Ok(match first.lock().unwrap().take() {
            Some(backend) => Box::new(backend) as Box<dyn Backend>,
            None => Box::new(FakeBackend::new(
                online(),
                Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            )),
        })
    });
    let handle = spawn_manager(
        factory,
        Arc::new(Net::default()),
        RetryPolicy::default(),
        tmp.path().to_path_buf(),
    );
    (tmp, handle, stopped, answered, finish)
}
#[tokio::test(start_paused = true)]
async fn replace_during_input_timeout_starts_after_clean_exit() {
    let (_tmp, handle, stopped, answered, finish) = stop_probe(true);
    handle.connect(params(false), 1).await.unwrap();
    wait_state(&handle, State::NeedsInput).await;
    tokio::time::advance(Duration::from_secs(120)).await;
    stopped.await.unwrap();
    assert!(answered.await.unwrap());
    assert_eq!(handle.status().await.state, State::Disconnecting);
    let replacement = tokio::spawn({
        let handle = handle.clone();
        async move { handle.connect(params(true), 1).await }
    });
    // A status command is a barrier behind the queued connect.
    tokio::task::yield_now().await;
    assert_eq!(handle.status().await.state, State::Disconnecting);
    assert!(!replacement.is_finished());
    finish.send(()).unwrap();
    assert_eq!(replacement.await.unwrap().unwrap().session, 2);
    wait_state(&handle, State::Connected).await;
    handle.disconnect(1).await;
}

#[tokio::test(start_paused = true)]
async fn repeated_state_publications_redact_detail_only_once() {
    let mut script = vec![FakeStep::Emit(FakeEvent::Progress("t".into()))];
    script.extend(online());
    for n in 0..3 {
        script.extend([
            FakeStep::Emit(FakeEvent::Restarting("restart".into())),
            FakeStep::Emit(FakeEvent::NetDown),
            FakeStep::Emit(FakeEvent::NetUp(config(if n == 2 {
                "10.0.0.9"
            } else {
                "10.0.0.1"
            }))),
            up(),
        ]);
    }
    let h = Harness::new(vec![script]);
    let mut events = h.handle.events();
    let mut p = params(false);
    if let BackendParams::Wireguard(wg) = &mut p.backend {
        wg.private_key = Secret::new("t");
    }
    h.handle.connect(p, 1).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Event::State(s) = events.recv().await.unwrap() {
                if let Some(detail) = &s.detail {
                    assert_eq!(detail, "[redatto]");
                }
                if s.state == State::Connected
                    && s.net
                        .as_ref()
                        .is_some_and(|n| n.dns_servers[0].to_string() == "10.0.0.9")
                {
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(h.handle.status().await.detail.as_deref(), Some("[redatto]"));
    h.handle.disconnect(1).await;
}
#[tokio::test(start_paused = true)]
async fn repeated_state_publications_redact_prompt_only_once() {
    let h = Harness::new(vec![vec![
        FakeStep::Emit(FakeEvent::InputRequired(InputRequest {
            request: 9,
            kind: InputKind::Otp,
            prompt: "t".into(),
            echo: false,
            expires_in_secs: 120,
        })),
        FakeStep::Emit(FakeEvent::NetUp(config("10.0.0.1"))),
    ]]);
    let mut p = params(false);
    if let BackendParams::Wireguard(wg) = &mut p.backend {
        wg.private_key = Secret::new("t");
    }
    h.handle.connect(p, 1).await.unwrap();
    let s = loop {
        let s = h.handle.status().await;
        if s.net.is_some() {
            break s;
        }
    };
    assert_eq!(s.input_request.unwrap().prompt, "[redatto]");
    h.handle.disconnect(1).await;
}
