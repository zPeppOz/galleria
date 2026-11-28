#![cfg(unix)]
//! What closing the window and quitting do (plan B6, T3): the decision with its table, the unsaved bit that stops both, the commands that
//! answer `{ done }`, and the question «is there a tray to come back from» on a bus. The event of the close itself (the «X») cannot be sent on
//! the mock runtime: the smoke test of the window sends the request a window manager would (`G10c`, `G10e`, `G10f`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "support/seed.rs"]
mod seed;
mod support;
#[path = "support/world.rs"]
mod world;

use galleria_gui::{
    closing::{Closing, Decision, Intent, decide},
    events::{CloseBlocked, EV_CLOSE_BLOCKED},
};
use rstest::rstest;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Listener;
use world::{Options, TestWindow, World, start};

#[rstest]
#[case(Intent::Hide, false, true, Decision::Hide)]
#[case(Intent::Hide, false, false, Decision::Quit)]
#[case(Intent::Hide, true, true, Decision::Blocked(Intent::Hide))]
#[case(Intent::Hide, true, false, Decision::Blocked(Intent::Quit))]
#[case(Intent::Quit, false, true, Decision::Quit)]
#[case(Intent::Quit, false, false, Decision::Quit)]
#[case(Intent::Quit, true, true, Decision::Blocked(Intent::Quit))]
#[case(Intent::Quit, true, false, Decision::Blocked(Intent::Quit))]
fn decide_table(
    #[case] intent: Intent,
    #[case] unsaved: bool,
    #[case] tray_shown: bool,
    #[case] expected: Decision,
) {
    assert_eq!(decide(intent, unsaved, tray_shown), expected);
}

#[test]
fn without_a_tray_host_a_close_is_a_quit() {
    let window = TestWindow::new(false);
    let closing = Closing::new(|| false);
    assert!(closing.request(&*window, Intent::Hide));
    assert_eq!((window.quits(), window.hides()), (1, 0));
    // with a tray that is shown it is a hide, and the app goes on
    let shown = TestWindow::new(false);
    assert!(Closing::new(|| true).request(&*shown, Intent::Hide));
    assert_eq!((shown.quits(), shown.hides()), (0, 1));
}

#[test]
fn a_quit_does_not_ask_the_bus_whether_the_tray_is_shown() {
    let window = TestWindow::new(false);
    // asking would panic: a quit does not depend on the tray
    let closing = Closing::new(|| panic!("the tray was asked about for a quit"));
    assert!(closing.request(&*window, Intent::Quit));
    assert_eq!(window.quits(), 1);
}

#[test]
fn unsaved_changes_block_both_the_close_and_the_quit() {
    let window = TestWindow::new(false);
    let closing = Closing::new(|| true);
    closing.set_unsaved(true);
    assert!(!closing.request(&*window, Intent::Hide));
    assert!(!closing.request(&*window, Intent::Quit));
    assert_eq!(window.blocked(), [Intent::Hide, Intent::Quit]);
    assert_eq!((window.quits(), window.hides()), (0, 0));
    // without a tray the close is a quit, and it is the quit that is asked about
    let alone = TestWindow::new(false);
    let no_tray = Closing::new(|| false);
    no_tray.set_unsaved(true);
    assert!(!no_tray.request(&*alone, Intent::Hide));
    assert_eq!(alone.blocked(), [Intent::Quit]);
    // when the page says there is nothing left, both go through
    closing.set_unsaved(false);
    assert!(closing.request(&*window, Intent::Hide));
    assert_eq!(window.hides(), 1);
}

#[test]
fn the_event_of_a_blocked_close_carries_the_intent_and_nothing_else() {
    assert_eq!(EV_CLOSE_BLOCKED, "galleria:close-blocked");
    assert_eq!(
        serde_json::to_value(CloseBlocked {
            intent: Intent::Hide
        })
        .unwrap(),
        json!({ "intent": "hide" })
    );
    assert_eq!(
        serde_json::to_value(CloseBlocked {
            intent: Intent::Quit
        })
        .unwrap(),
        json!({ "intent": "quit" })
    );
}

fn start_with(window: &Arc<TestWindow>, closing: Closing) -> World {
    start(Options {
        window: Some(Arc::clone(window) as Arc<dyn galleria_gui::window::WindowControl>),
        closing: Some(Arc::new(closing)),
        ..Options::default()
    })
}

#[test]
fn a_close_without_unsaved_changes_does_not_wait_for_the_page() {
    // there is no page in this app: nothing answers, and the window is hidden at once all the same
    let window = TestWindow::new(false);
    let world = start_with(&window, Closing::new(|| true));
    let begun = Instant::now();
    assert_eq!(
        world.invoke("window_hide", json!({})).unwrap(),
        json!({ "done": true })
    );
    assert!(
        begun.elapsed() < Duration::from_millis(200),
        "{:?}",
        begun.elapsed()
    );
    assert_eq!(window.hides(), 1);
}

#[test]
fn window_hide_and_app_quit_go_through_the_same_decision() {
    let window = TestWindow::new(false);
    let world = start_with(&window, Closing::new(|| true));
    assert_eq!(
        world
            .invoke("unsaved_set", json!({ "dirty": true }))
            .unwrap(),
        Value::Null
    );
    assert_eq!(
        world.invoke("window_hide", json!({})).unwrap(),
        json!({ "done": false })
    );
    assert_eq!(
        world.invoke("app_quit", json!({})).unwrap(),
        json!({ "done": false })
    );
    assert_eq!(window.blocked(), [Intent::Hide, Intent::Quit]);
    assert_eq!((window.hides(), window.quits()), (0, 0));
    // the page was asked, and answered «Discard»: the bit goes down and both are done
    world
        .invoke("unsaved_set", json!({ "dirty": false }))
        .unwrap();
    assert_eq!(
        world.invoke("window_hide", json!({})).unwrap(),
        json!({ "done": true })
    );
    assert_eq!(
        world.invoke("app_quit", json!({})).unwrap(),
        json!({ "done": true })
    );
    assert_eq!((window.hides(), window.quits()), (1, 1));
}

#[test]
fn a_tray_quit_is_the_same_decision() {
    use galleria_gui::{tray::TrayAction, tray_tauri::run_action};
    let window = TestWindow::new(false);
    let world = start_with(&window, Closing::new(|| true));
    world
        .invoke("unsaved_set", json!({ "dirty": true }))
        .unwrap();
    run_action(world.app(), &TrayAction::Quit);
    assert_eq!((window.blocked(), window.quits()), (vec![Intent::Quit], 0));
    world
        .invoke("unsaved_set", json!({ "dirty": false }))
        .unwrap();
    run_action(world.app(), &TrayAction::Quit);
    assert_eq!(window.quits(), 1);
}

#[test]
fn the_real_window_tells_the_page_what_was_blocked() {
    // the app's own window control (no test window): the event of the mock app is heard with a Rust listener
    let world = start(Options {
        closing: Some(Arc::new(Closing::new(|| true))),
        ..Options::default()
    });
    let heard: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink = Arc::clone(&heard);
    world.app().listen(EV_CLOSE_BLOCKED, move |event| {
        sink.lock()
            .unwrap()
            .push(serde_json::from_str(event.payload()).unwrap());
    });
    world
        .invoke("unsaved_set", json!({ "dirty": true }))
        .unwrap();
    assert_eq!(
        world.invoke("app_quit", json!({})).unwrap(),
        json!({ "done": false })
    );
    let begun = Instant::now();
    while heard.lock().unwrap().is_empty() {
        assert!(
            begun.elapsed() < Duration::from_secs(8),
            "the event never came"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(heard.lock().unwrap()[0], json!({ "intent": "quit" }));
}

// ---- is there a tray to come back from? (S51) ------------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn tray_is_shown_is_false_without_a_bus_and_without_an_owner() {
    use galleria_gui::tray_host::tray_is_shown_on;
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
    };
    // no bus: no address, an empty one, one that is not a bus
    assert!(!tray_is_shown_on(None));
    assert!(!tray_is_shown_on(Some("")));
    assert!(!tray_is_shown_on(Some("unix:path=/nonexistent/no-bus")));
    // a bus with nobody that owns `org.kde.StatusNotifierWatcher`: a private session bus, as the image has
    let mut daemon = Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("dbus-daemon must be in the image: this proof is not skipped");
    let mut address = String::new();
    BufReader::new(daemon.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    let shown = tray_is_shown_on(Some(address.trim()));
    let _ = daemon.kill();
    let _ = daemon.wait();
    assert!(
        !address.trim().is_empty(),
        "the private bus gave no address"
    );
    assert!(
        !shown,
        "a bus without a watcher was taken for a tray that is shown"
    );
}
