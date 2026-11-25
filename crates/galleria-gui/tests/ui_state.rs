//! `state.toml` (D32, Q25, Q34): the pure module and, on the mock runtime, the two commands that expose it.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleria_gui::{
    configure_with,
    supervisor::RelinkPolicy,
    ui_state::{
        LangChoice, STATE_SCHEMA, SortOrder, ThemeChoice, UiPatch, UiState, UiStateFile,
        WindowSize, state_path,
    },
};
use rstest::rstest;
use serde_json::{Value, json};
use std::{ffi::OsString, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};
use time::OffsetDateTime;
use uuid::Uuid;

fn file_in(dir: &tempfile::TempDir) -> UiStateFile {
    UiStateFile::new(dir.path().join("state/galleria/state.toml"))
}

#[test]
fn missing_file_gives_the_defaults_without_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(file_in(&dir).load(), UiState::default());
    assert_eq!(UiState::default().schema_version, STATE_SCHEMA);
}

#[test]
fn broken_toml_gives_the_defaults_and_the_next_write_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    std::fs::create_dir_all(file.path().parent().unwrap()).unwrap();
    std::fs::write(file.path(), "this is [not toml").unwrap();
    assert_eq!(file.load(), UiState::default());
    file.update(|s| s.theme = ThemeChoice::Light).unwrap();
    assert_eq!(file.load().theme, ThemeChoice::Light);
    // a value outside the choices is as broken as a syntax error
    std::fs::write(file.path(), "lang = \"klingon\"\n").unwrap();
    assert_eq!(file.load(), UiState::default());
}

#[test]
fn newer_schema_gives_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    std::fs::create_dir_all(file.path().parent().unwrap()).unwrap();
    std::fs::write(file.path(), "schema_version = 99\ntheme = \"light\"\n").unwrap();
    assert_eq!(file.load(), UiState::default());
}

#[test]
fn unknown_keys_are_tolerated() {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    std::fs::create_dir_all(file.path().parent().unwrap()).unwrap();
    std::fs::write(
        file.path(),
        "theme = \"dark\"\nsomething_new = 3\n[panel]\nwidth = 9\n",
    )
    .unwrap();
    assert_eq!(file.load().theme, ThemeChoice::Dark);
    // they are lost at the first write: the file is one that costs nothing to lose
    file.update(|s| s.lang = LangChoice::It).unwrap();
    let text = std::fs::read_to_string(file.path()).unwrap();
    assert!(!text.contains("something_new"), "{text}");
    assert!(text.contains("theme = \"dark\""), "{text}");
}

#[test]
fn state_dir_and_file_are_private() {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    assert!(!file.path().parent().unwrap().exists());
    file.update(|s| s.sort = SortOrder::Recent).unwrap();
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(file.path().parent().unwrap()), 0o700);
    assert_eq!(mode(file.path()), 0o600);
    // a second write keeps both
    file.update(|s| s.sort = SortOrder::Name).unwrap();
    assert_eq!(mode(file.path()), 0o600);
    // and leaves no temporary file behind
    let left: Vec<_> = std::fs::read_dir(file.path().parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, vec![OsString::from("state.toml")]);
}

#[test]
fn update_keeps_what_it_does_not_touch() {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    let id = Uuid::new_v4();
    file.touch_last_used(id, OffsetDateTime::UNIX_EPOCH)
        .unwrap();
    file.update(|s| {
        s.window = Some(WindowSize {
            width: 900,
            height: 700,
            maximized: true,
        });
    })
    .unwrap();
    let after = file.update(|s| s.theme = ThemeChoice::Dark).unwrap();
    assert_eq!(after.theme, ThemeChoice::Dark);
    let loaded = file.load();
    assert_eq!(loaded, after);
    assert_eq!(loaded.last_used[&id], "1970-01-01T00:00:00Z");
    assert_eq!(loaded.window.unwrap().width, 900);
    assert!(loaded.window.unwrap().maximized);
}

#[rstest]
#[case(Some("/run/state"), Some("/home/u"), "/run/state/galleria/state.toml")]
#[case(None, Some("/home/u"), "/home/u/.local/state/galleria/state.toml")]
#[case(Some(""), Some("/home/u"), "/home/u/.local/state/galleria/state.toml")]
fn path_follows_xdg_state_home_and_falls_back_to_home(
    #[case] xdg: Option<&str>,
    #[case] home: Option<&str>,
    #[case] expected: &str,
) {
    assert_eq!(
        state_path(xdg.map(OsString::from), home.map(OsString::from)),
        PathBuf::from(expected)
    );
}

#[test]
fn touch_and_forget_last_used() {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    file.touch_last_used(a, OffsetDateTime::UNIX_EPOCH).unwrap();
    file.touch_last_used(b, OffsetDateTime::UNIX_EPOCH + time::Duration::days(1))
        .unwrap();
    file.touch_last_used(a, OffsetDateTime::UNIX_EPOCH + time::Duration::days(2))
        .unwrap();
    let used = file.load().last_used;
    assert_eq!(used[&a], "1970-01-03T00:00:00Z");
    assert_eq!(used[&b], "1970-01-02T00:00:00Z");
    file.forget(a).unwrap();
    file.forget(Uuid::new_v4()).unwrap();
    let used = file.load().last_used;
    assert!(!used.contains_key(&a) && used.contains_key(&b));
}

#[test]
fn a_maximised_window_keeps_the_size_to_come_back_to() {
    let before = Some(WindowSize {
        width: 1000,
        height: 700,
        maximized: false,
    });
    let maximised = WindowSize::after(before, 1920, 1080, true);
    assert_eq!((maximised.width, maximised.height), (1000, 700));
    assert!(maximised.maximized);
    let restored = WindowSize::after(Some(maximised), 1100, 800, false);
    assert_eq!((restored.width, restored.height), (1100, 800));
    assert!(!restored.maximized);
    // with nothing saved yet the current size is all there is
    assert_eq!(WindowSize::after(None, 1920, 1080, true).width, 1920);
}

#[test]
fn an_unusable_saved_size_is_not_applied() {
    let size = |width, height| WindowSize {
        width,
        height,
        maximized: false,
    };
    assert!(size(1180, 740).usable().is_some());
    assert!(size(0, 740).usable().is_none());
    assert!(size(1180, 30).usable().is_none());
    assert!(size(1_000_000, 740).usable().is_none());
}

// ---- the patch and the commands --------------------------------------------------------------------------------------

#[test]
fn patch_cannot_set_window_or_last_used() {
    for body in [
        json!({"window": {"width": 1, "height": 1, "maximized": true}}),
        json!({"last_used": {}}),
        json!({"schema_version": 7}),
        json!({"lang": "it", "window": null}),
    ] {
        assert!(
            serde_json::from_value::<UiPatch>(body.clone()).is_err(),
            "{body} was accepted"
        );
    }
    let patch: UiPatch = serde_json::from_value(json!({"lang": "it", "sort": "recent"})).unwrap();
    let mut state = UiState::default();
    patch.apply(&mut state);
    assert_eq!(
        (state.lang, state.theme, state.sort),
        (LangChoice::It, ThemeChoice::System, SortOrder::Recent)
    );
}

fn app(file: UiStateFile) -> (tauri::App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let mut app = configure_with(
        mock_builder(),
        std::env::temp_dir().join("galleria-no-daemon.sock"),
        RelinkPolicy {
            delays: vec![Duration::from_millis(20)],
        },
        file,
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    (app, window)
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn commands_round_trip_through_the_fake_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("galleria/state.toml");
    let (_app, window) = app(UiStateFile::new(path.clone()));

    let first = invoke(&window, "ui_state_get", json!({})).unwrap();
    assert_eq!(first["lang"], "system");
    assert_eq!(first["theme"], "system");
    assert_eq!(first["sort"], "name");

    let set = invoke(
        &window,
        "ui_state_set",
        json!({"patch": {"lang": "it", "theme": "light"}}),
    )
    .unwrap();
    assert_eq!(
        (&set["lang"], &set["theme"]),
        (&json!("it"), &json!("light"))
    );

    let again = invoke(&window, "ui_state_get", json!({})).unwrap();
    assert_eq!(
        (&again["lang"], &again["theme"], &again["sort"]),
        (&json!("it"), &json!("light"), &json!("name"))
    );
    // what the webview chose is on the disk, so a new process finds it
    assert_eq!(UiStateFile::new(path).load().lang, LangChoice::It);

    // the window and the last use are not in the patch
    let refused = invoke(
        &window,
        "ui_state_set",
        json!({"patch": {"window": {"width": 1, "height": 1, "maximized": false}}}),
    );
    assert!(refused.is_err(), "{refused:?}");
}

#[test]
fn a_failed_write_is_an_error_with_a_code() {
    let dir = tempfile::tempdir().unwrap();
    // the directory of the state file is a file: the write cannot happen
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "x").unwrap();
    let (_app, window) = app(UiStateFile::new(blocker.join("state.toml")));
    let err = invoke(&window, "ui_state_set", json!({"patch": {"theme": "dark"}})).unwrap_err();
    assert_eq!(err["code"], "gui_state_not_saved");
    // reading still works
    assert_eq!(
        invoke(&window, "ui_state_get", json!({})).unwrap()["theme"],
        "system"
    );
}
