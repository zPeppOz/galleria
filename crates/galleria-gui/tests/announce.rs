//! Q27: what the screen reader is told. The pure choices, the command on the mock runtime (which has no GTK window:
//! `gtk_window()` is `unimplemented!()` there, so the command talks to a trait and these tests use one that records),
//! and, on Linux, the emission of the real ATK signal. The road from the webview to Orca is tried in the image
//! (`scripts/gui.sh a11y`, `S6b` and `S6c`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleria_gui::{
    announce::{Announcer, MAX_LEN, Signal, Urgency, clean, pick_signal},
    configure_with,
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use rstest::rstest;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};

#[rstest]
#[case("a\u{7}b", Some("ab"))]
#[case("first\nsecond\r\n\tthird", Some("first second third"))]
#[case("  spaced   out  ", Some("spaced out"))]
#[case("   \n\t ", None)]
#[case("\u{7}\u{1b}", None)]
#[case("", None)]
#[case(
    "Saved «Ufficio Napoli» — è fatto",
    Some("Saved «Ufficio Napoli» — è fatto")
)]
fn clean_strips_control_characters_and_caps_the_length(
    #[case] raw: &str,
    #[case] expected: Option<&str>,
) {
    assert_eq!(clean(raw).as_deref(), expected);
}

#[test]
fn clean_caps_a_long_text_at_the_limit() {
    assert_eq!(MAX_LEN, 300);
    let long = "x".repeat(500);
    assert_eq!(clean(&long).unwrap().chars().count(), 300);
    // characters, not bytes: 400 accented letters stay 300 letters
    assert_eq!(clean(&"è".repeat(400)).unwrap().chars().count(), 300);
    // and a cut never leaves a space at the end
    let text = format!("{} {}", "y".repeat(299), "z".repeat(50));
    assert_eq!(clean(&text).unwrap(), "y".repeat(299));
}

#[rstest]
#[case(true, true, Urgency::Polite, Some(Signal::Notification(1)))]
#[case(true, true, Urgency::Assertive, Some(Signal::Notification(2)))]
#[case(true, false, Urgency::Polite, Some(Signal::Notification(1)))]
#[case(false, true, Urgency::Polite, Some(Signal::Announcement))]
#[case(false, true, Urgency::Assertive, Some(Signal::Announcement))]
#[case(false, false, Urgency::Polite, None)]
#[case(false, false, Urgency::Assertive, None)]
fn pick_signal_follows_the_table(
    #[case] notification: bool,
    #[case] announcement: bool,
    #[case] urgency: Urgency,
    #[case] expected: Option<Signal>,
) {
    assert_eq!(pick_signal(notification, announcement, urgency), expected);
}

// ---- the command ----------------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<(String, Urgency)>>>);

impl Announcer for Recorder {
    fn announce(&self, text: &str, urgency: Urgency) {
        self.0.lock().unwrap().push((text.to_owned(), urgency));
    }
}

impl Recorder {
    fn heard(&self) -> Vec<(String, Urgency)> {
        self.0.lock().unwrap().clone()
    }
}

fn app(
    recorder: &Recorder,
) -> (
    tauri::App<MockRuntime>,
    WebviewWindow<MockRuntime>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let announcer: Arc<dyn Announcer> = Arc::new(recorder.clone());
    let mut app = configure_with(
        // managed before the build: the setup of `configure_with` keeps this one and not the ATK one
        mock_builder().manage(announcer),
        dir.path().join("galleriad.sock"),
        RelinkPolicy {
            delays: vec![Duration::from_millis(20)],
        },
        UiStateFile::new(dir.path().join("state/state.toml")),
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    (app, window, dir)
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn announce_command_forwards_the_cleaned_text_and_the_urgency() {
    let recorder = Recorder::default();
    let (_app, window, _dir) = app(&recorder);
    invoke(
        &window,
        "announce",
        json!({"text": "  Saved\u{7}\n«Homelab» ", "assertive": false}),
    )
    .unwrap();
    invoke(
        &window,
        "announce",
        json!({"text": "Could not save", "assertive": true}),
    )
    .unwrap();
    assert_eq!(
        recorder.heard(),
        vec![
            ("Saved «Homelab»".to_owned(), Urgency::Polite),
            ("Could not save".to_owned(), Urgency::Assertive),
        ]
    );
}

#[test]
fn empty_text_is_dropped_without_an_error() {
    let recorder = Recorder::default();
    let (_app, window, _dir) = app(&recorder);
    for text in ["", "   ", "\u{7}\n"] {
        let answer = invoke(
            &window,
            "announce",
            json!({"text": text, "assertive": true}),
        );
        assert!(answer.is_ok(), "{text:?}: {answer:?}");
    }
    assert!(recorder.heard().is_empty());
}

#[test]
fn announce_takes_a_text_and_a_flag_and_nothing_else() {
    // no path, no secret, no identifier of anything: the webview can make Orca say a sentence, nothing more
    let recorder = Recorder::default();
    let (_app, window, _dir) = app(&recorder);
    let answer = invoke(&window, "announce", json!({"text": "x"}));
    assert!(answer.is_err(), "`assertive` is required: {answer:?}");
    assert!(recorder.heard().is_empty());
}

// ---- the real ATK signal (Linux) -------------------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod atk {
    use galleria_gui::{
        announce::{Signal, Urgency},
        announce_atk::emit_on,
    };
    use gtk::{
        atk::NoOpObject,
        glib::{self, StaticType, object::ObjectExt, subclass::SignalId},
        prelude::Cast,
    };
    use std::sync::{Arc, Mutex, MutexGuard};

    /// Every test of this module takes it first. GLib registers the ATK types (`AtkNoOpObject` and its interfaces) the
    /// first time one is asked for, and that is not safe from two threads at once: the tests run on one thread each,
    /// and two of them at the same time make `g_type_add_interface_static` fail and `NoOpObject::new` panic (about 1
    /// run in 10). The app is not exposed: it only touches ATK on the main thread of GTK (`AtkAnnouncer`).
    /// A test that panicked while holding it leaves the lock poisoned, and the next ones must still run.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn one_at_a_time() -> MutexGuard<'static, ()> {
        ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn a_missing_signal_is_skipped_without_a_panic() {
        let _one = one_at_a_time();
        // A plain GObject has neither `announcement` nor `notification`, like the accessible of a system with an
        // ATK older than 2.46: `emit_by_name` would panic, so the signal is looked up first (S19).
        let plain = glib::Object::new::<glib::Object>();
        assert!(SignalId::lookup("galleria-no-such-signal", plain.type_()).is_none());
        assert!(SignalId::lookup("announcement", plain.type_()).is_none());
        assert!(SignalId::lookup("notification", plain.type_()).is_none());
        assert_eq!(emit_on(&plain, "nobody hears this", Urgency::Polite), None);
        assert_eq!(emit_on(&plain, "nor this", Urgency::Assertive), None);
    }

    #[test]
    fn the_signal_atk_has_is_emitted_with_the_text_and_the_urgency() {
        let _one = one_at_a_time();
        // A real AtkObject, no display needed. The signal names and arguments are those of ATK ≥ 2.50 (the image has 2.52).
        let accessible = NoOpObject::new(&glib::Object::new::<glib::Object>());
        assert!(
            SignalId::lookup("notification", accessible.type_()).is_some(),
            "this ATK has no `notification`"
        );
        let heard: Arc<Mutex<Vec<(String, i32)>>> = Arc::default();
        let sink = Arc::clone(&heard);
        accessible.connect_local("notification", false, move |args| {
            let text = args[1].get::<String>().unwrap();
            let urgency = args[2].get::<i32>().unwrap();
            sink.lock().unwrap().push((text, urgency));
            None
        });
        assert_eq!(
            emit_on(accessible.upcast_ref(), "Saved", Urgency::Polite),
            Some(Signal::Notification(1))
        );
        assert_eq!(
            emit_on(
                accessible.upcast_ref(),
                "Could not save",
                Urgency::Assertive
            ),
            Some(Signal::Notification(2))
        );
        assert_eq!(
            *heard.lock().unwrap(),
            vec![("Saved".to_owned(), 1), ("Could not save".to_owned(), 2)]
        );
    }

    #[test]
    fn the_type_of_an_atk_object_is_not_a_plain_gobject() {
        let _one = one_at_a_time();
        // control for the test above: it is the lookup on the type that tells the two apart
        assert_ne!(NoOpObject::static_type(), glib::Object::static_type());
    }
}

// ---- the dependency ---------------------------------------------------------------------------------------------------

fn crate_file(path: &str) -> String {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

#[test]
fn gtk_is_the_one_tauri_uses() {
    // `gtk` 0.18 is named by galleria-gui for the announcement: it must stay the version of tauri, tao, wry and muda,
    // or the tree would carry two GTK3 bindings. When Tauri leaves GTK3 this and the two entries of deny.toml go.
    let lock = crate_file("../../Cargo.lock");
    let versions: Vec<&str> = lock
        .split("[[package]]")
        .filter(|entry| entry.lines().any(|line| line == "name = \"gtk\""))
        .map(|entry| entry.lines().find(|l| l.starts_with("version = ")).unwrap())
        .collect();
    assert_eq!(
        versions.len(),
        1,
        "Cargo.lock has these gtk versions: {versions:?}"
    );
    assert!(
        versions[0].starts_with("version = \"0.18."),
        "{}",
        versions[0]
    );
}

#[test]
fn no_glib_or_atk_direct_dependency() {
    // deny.toml accepts RUSTSEC-2024-0429 because Galleria names no glib crate itself: they come through `gtk`.
    for line in crate_file("Cargo.toml").lines() {
        let key = line.trim().split(['=', ' ']).next().unwrap_or("");
        assert!(
            !matches!(key, "glib" | "atk" | "gdk" | "pango" | "cairo-rs")
                && !key.starts_with("glib-")
                && !key.starts_with("atk-"),
            "galleria-gui names a GTK binding crate directly: {line}"
        );
    }
}
