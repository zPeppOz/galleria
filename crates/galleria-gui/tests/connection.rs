//! Connecting from the window (plan B5, T4): `connect_profile` and `disconnect` against a real daemon with a fake backend, the
//! replacement of a session, the last use of a profile, the announcements of the state, and that no command, no event and no
//! line of `stderr` carries a secret. The commands are invoked as the webview does; the events are heard with a Rust listener.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;
mod support;
#[path = "support/world.rs"]
mod world;

use galleria_gui::{
    announce::Urgency, events::TauriSink, rubrica::SecretFactory, supervisor::EventSink,
};
use galleria_ipc::{Secret, Status};
use galleria_profile::{FileSecretStore, ProfileStore, SecretStore, StoreError, to_connect_params};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use support::*;
use uuid::Uuid;
use world::*;

#[test]
fn connect_profile_sends_what_the_cli_would_send() {
    let world = start(Options::default());
    for name in ["Homelab", "Ufficio Napoli"] {
        let id = world.id(name);
        world.connect(name, false).unwrap();
        world.until_state("connected", Some(id));
        let store = ProfileStore::open(&world.secrets_dir()).unwrap();
        let profile = store.find(&id.to_string()).unwrap();
        let secrets = FileSecretStore::open(&world.secrets_dir()).unwrap();
        let cli = to_connect_params(&profile, &secrets, false).unwrap();
        let sent = world
            .daemon()
            .harness
            .connects
            .lock()
            .unwrap()
            .last()
            .cloned()
            .unwrap();
        assert_eq!(sent["backend"], serde_json::to_value(&cli.backend).unwrap());
        assert_eq!(sent["options"], serde_json::to_value(&cli.options).unwrap());
        world.invoke("disconnect", json!({})).unwrap();
        world.until_state("disconnected", None);
    }
    world.stop();
}

#[test]
fn connect_reaches_connected_through_the_events() {
    let world = start(Options::default());
    let id = world.id("Homelab");
    assert_eq!(world.connect("Homelab", false).unwrap(), Value::Null);
    world.until_state("connected", Some(id));
    let states = world.states();
    let connecting = states.iter().position(|s| s == "connecting").unwrap();
    let connected = states.iter().position(|s| s == "connected").unwrap();
    assert!(connecting < connected, "{states:?}");
    world.stop();
}

#[test]
fn disconnect_returns_to_disconnected() {
    let world = start(Options::default());
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    assert_eq!(world.invoke("disconnect", json!({})).unwrap(), Value::Null);
    world.until_state("disconnected", None);
    assert_eq!(world.states().last().unwrap(), "disconnected");
    world.stop();
}

#[test]
fn disconnect_without_a_session_is_not_an_error_for_the_page() {
    let world = start(Options::default());
    // the daemon ends nothing and says it is done (it has no `not_active` for this): the page sees `()`, like for a session
    assert_eq!(world.invoke("disconnect", json!({})).unwrap(), Value::Null);
    world.stop();
}

#[test]
fn a_second_profile_of_the_rubrica_is_already_active_and_not_foreign() {
    let world = start(Options::default());
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    let error = world.connect("VPS Hetzner", false).unwrap_err();
    assert_eq!(code(&error), "already_active");
    assert_eq!(error["data"]["foreign"], false);
    assert_eq!(error["data"]["profile"]["name"], "Homelab");
    assert_eq!(
        error["data"]["profile"]["id"],
        world.id("Homelab").to_string()
    );
    world.stop();
}

#[test]
fn a_session_outside_the_rubrica_is_foreign() {
    let world = start(Options::default());
    // another client of the daemon starts a session with an id that this rubrica does not have (G9)
    world.rt.block_on(async {
        let mut other = world.daemon().client().await;
        let _: Value = other.call("connect", &params(false)).await.unwrap();
    });
    world.until_state("connected", None);
    let error = world.connect("Homelab", false).unwrap_err();
    assert_eq!(code(&error), "already_active");
    assert_eq!(error["data"]["foreign"], true);
    assert_eq!(error["data"]["profile"]["name"], "Test VPN");
    world.stop();
}

/// An archive that has every key, for the test of a rubrica that stops being readable in the middle of a connection.
struct Everything;
impl SecretStore for Everything {
    fn get(&self, _: &Uuid, _: &str) -> Result<Option<Secret>, StoreError> {
        Ok(Some(Secret::new("a-key")))
    }
    fn set(&self, _: &Uuid, _: &str, _: &Secret) -> Result<(), StoreError> {
        Ok(())
    }
    fn delete(&self, _: &Uuid, _: &str) -> Result<(), StoreError> {
        Ok(())
    }
    fn delete_all(&self, _: &Uuid) -> Result<(), StoreError> {
        Ok(())
    }
}

#[test]
fn an_unreadable_rubrica_makes_the_session_foreign() {
    // Once `loosen` is set the next open of the archive opens the folder of the profiles to everyone, as a fix gone wrong in a
    // terminal would: the profile was read a moment before and the parameters are made, but the rubrica cannot answer
    // «is this session one of mine» any more.
    let loosen = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = Arc::clone(&loosen);
    let factory: SecretFactory = Arc::new(move |dir: &Path, _: &str| {
        if flag.load(std::sync::atomic::Ordering::SeqCst) {
            fs::set_permissions(dir, fs::Permissions::from_mode(0o777)).unwrap();
            Ok(Box::new(Everything) as Box<dyn SecretStore>)
        } else {
            Ok(Box::new(FileSecretStore::open(dir)?) as Box<dyn SecretStore>)
        }
    });
    let world = start(Options {
        secrets: Some(factory),
        ..Options::default()
    });
    let own = world
        .rubrica
        .connect_params(world.id("Homelab"), false)
        .unwrap();
    world.rt.block_on(async {
        let mut other = world.daemon().client().await;
        let _: Value = other.call("connect", &own).await.unwrap();
    });
    world.until_state("connected", Some(world.id("Homelab")));
    loosen.store(true, std::sync::atomic::Ordering::SeqCst);
    let error = world.connect("VPS Hetzner", false).unwrap_err();
    assert_eq!(code(&error), "already_active");
    // the session is Homelab, which the rubrica has: but it cannot be read, and in doubt it is asked about
    assert_eq!(error["data"]["foreign"], true);
    assert_eq!(error["data"]["profile"]["name"], "Homelab");
    world.stop();
}

#[test]
fn replace_true_swaps_the_session_and_never_runs_two_backends() {
    let world = start(Options::default());
    let first = world.id("Homelab");
    let second = world.id("VPS Hetzner");
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", Some(first));
    world.connect("VPS Hetzner", true).unwrap();
    world.until_state("connected", Some(second));
    let harness = &world.daemon().harness;
    // the fake backend asserts, when it starts, that none is alive: a second one before the first has gone would panic
    assert_eq!(harness.spawns.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(harness.live.load(std::sync::atomic::Ordering::SeqCst), 1);
    world.stop();
}

#[test]
fn disconnecting_a_foreign_session_is_not_automatic() {
    let world = start(Options::default());
    world.rt.block_on(async {
        let mut other = world.daemon().client().await;
        let _: Value = other.call("connect", &params(false)).await.unwrap();
    });
    world.until_state("connected", None);
    let error = world.connect("Homelab", false).unwrap_err();
    assert_eq!(error["data"]["foreign"], true);
    // nothing ended the session: it is up, and no backend was told to stop
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(world.states().last().unwrap(), "connected");
    let log = world.daemon().harness.net.log.lock().unwrap().clone();
    assert!(!log.iter().any(|line| line == "stop"), "{log:?}");
    world.stop();
}

#[test]
fn a_missing_key_is_gui_secret_missing() {
    let world = start(Options::default());
    let id = world.id("Homelab");
    FileSecretStore::open(&world.secrets_dir())
        .unwrap()
        .delete_all(&id)
        .unwrap();
    let error = world.connect("Homelab", false).unwrap_err();
    assert_eq!(code(&error), "gui_secret_missing");
    assert!(world.daemon().harness.connects.lock().unwrap().is_empty());
    world.stop();
}

#[test]
fn a_locked_keyring_is_reported_and_nothing_is_sent() {
    let locked: SecretFactory = Arc::new(|_: &Path, _: &str| Err(StoreError::KeyringLocked));
    let world = start(Options {
        secrets: Some(locked),
        ..Options::default()
    });
    let error = world.connect("Homelab", false).unwrap_err();
    assert_eq!(code(&error), "gui_keyring_locked");
    assert!(world.daemon().harness.connects.lock().unwrap().is_empty());
    world.stop();
}

#[test]
fn without_the_daemon_connect_is_gui_daemon_unreachable() {
    let world = start(Options {
        daemon: false,
        ..Options::default()
    });
    let error = world.connect("Homelab", false).unwrap_err();
    assert_eq!(code(&error), "gui_daemon_unreachable");
    let error = world.invoke("disconnect", json!({})).unwrap_err();
    assert_eq!(code(&error), "gui_daemon_unreachable");
}

#[test]
fn connected_touches_the_last_use_of_a_rubrica_profile_only() {
    let world = start(Options::default());
    let id = world.id("Homelab");
    // a session outside the rubrica: nothing is written
    world.rt.block_on(async {
        let mut other = world.daemon().client().await;
        let _: Value = other.call("connect", &params(false)).await.unwrap();
        let _: Value = other.call("disconnect", &json!({})).await.unwrap();
    });
    world.until_state("disconnected", None);
    assert!(!world.state_file().contains(&id.to_string()));
    assert!(!world.state_file().contains("test"));
    // a session of the rubrica: the last use of that profile
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", Some(id));
    let start = Instant::now();
    while !world.state_file().contains(&id.to_string()) {
        assert!(start.elapsed() < WAIT, "{}", world.state_file());
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !world
            .state_file()
            .contains(&world.id("VPS Hetzner").to_string())
    );
    world.stop();
}

fn spoken(world: &World) -> Vec<(String, Urgency)> {
    world.spoken.0.lock().unwrap().clone()
}

#[test]
fn state_changes_are_announced_once_each() {
    let world = start(Options::default());
    world.phrases();
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    // more events of the same state (the daemon repeats `connected` when something in it changes) are not more announcements:
    // the state the page was given is delivered three more times, as the supervisor does, and the sentence is still said once
    let connected: Status = {
        let events = world.events.lock().unwrap();
        let (_, value) = events
            .iter()
            .rev()
            .find(|(name, value)| name == "galleria:state" && value["state"] == "connected")
            .expect("the page was given the state `connected`");
        serde_json::from_value(value.clone()).unwrap()
    };
    let sink = TauriSink(world._app.handle().clone());
    for _ in 0..3 {
        sink.state(&connected);
    }
    world.invoke("disconnect", json!({})).unwrap();
    world.until_state("disconnected", None);
    let said = spoken(&world);
    assert_eq!(
        said,
        vec![
            ("Connected to Homelab".to_owned(), Urgency::Polite),
            ("Disconnected".to_owned(), Urgency::Polite),
        ],
        "{said:?}"
    );
    world.stop();
}

#[test]
fn an_error_is_announced_assertive_with_its_translated_text() {
    let world = start(Options {
        scenario: "unreachable",
        ..Options::default()
    });
    world.phrases();
    world.connect("Homelab", false).unwrap();
    world.until_state("error", None);
    let start = Instant::now();
    while spoken(&world).is_empty() {
        assert!(start.elapsed() < WAIT);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        spoken(&world),
        vec![(
            "Cannot reach the server of Homelab".to_owned(),
            Urgency::Assertive
        )]
    );
    world.stop();
}

#[test]
fn connecting_and_needs_input_are_not_announced() {
    let world = start(Options {
        scenario: "credentials",
        ..Options::default()
    });
    world.phrases();
    world.connect("Cliente Rossi", false).unwrap();
    world.until_state("needs_input", None);
    assert!(world.states().contains(&"connecting".to_owned()));
    assert_eq!(spoken(&world), vec![]);
    world.stop();
}

#[test]
fn no_phrases_no_announcement() {
    let world = start(Options::default());
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    world.invoke("disconnect", json!({})).unwrap();
    world.until_state("disconnected", None);
    assert_eq!(spoken(&world), vec![]);
    world.stop();
}

#[test]
fn a_phrase_is_cleaned_and_capped_before_it_is_said() {
    let world = start(Options::default());
    world
        .invoke(
            "announce_phrases",
            json!({ "phrases": {
                "connected": format!("Up\n{{name}}\u{7} {}", "x".repeat(400)),
                "disconnected": "Down",
                "reconnecting": "Again",
                "error": {},
                "error_other": "Error",
            }}),
        )
        .unwrap();
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    let start = Instant::now();
    while spoken(&world).is_empty() {
        assert!(start.elapsed() < WAIT);
        std::thread::sleep(Duration::from_millis(5));
    }
    let (text, _) = spoken(&world).remove(0);
    assert!(text.starts_with("Up Homelab "), "{text}");
    assert!(text.chars().count() <= 300);
    assert!(!text.contains('\u{7}') && !text.contains('\n'));
    world.stop();
}

fn secrets_of_the_seed() -> Vec<String> {
    vec![seed::SECRET_MARK.to_owned()]
}

#[test]
fn no_command_returns_a_secret() {
    let world = start(Options::default());
    let mut seen: Vec<(String, String)> = vec![];
    let mut note = |what: &str, answer: Result<Value, Value>| {
        let value = answer.unwrap_or_else(|e| e);
        seen.push((what.to_owned(), value.to_string()));
        value
    };
    note("connect_profile", world.connect("Homelab", false));
    world.until_state("connected", None);
    note("connect_profile again", world.connect("Homelab", false));
    note(
        "connect_profile other",
        world.connect("Ufficio Napoli", false),
    );
    note("daemon_view", world.invoke("daemon_view", json!({})));
    note("disconnect", world.invoke("disconnect", json!({})));
    world.until_state("disconnected", None);
    note("disconnect again", world.invoke("disconnect", json!({})));
    FileSecretStore::open(&world.secrets_dir())
        .unwrap()
        .delete_all(&world.id("Homelab"))
        .unwrap();
    note(
        "connect_profile without keys",
        world.connect("Homelab", false),
    );
    for (what, text) in &seen {
        for secret in secrets_of_the_seed() {
            assert!(!text.contains(&secret), "{what} returned a secret: {text}");
        }
    }
    world.stop();
}

#[test]
fn no_event_carries_a_secret() {
    let world = start(Options::default());
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    world.connect("Ufficio Napoli", true).unwrap();
    world.until_state("connected", Some(world.id("Ufficio Napoli")));
    world.invoke("disconnect", json!({})).unwrap();
    world.until_state("disconnected", None);
    let events = world.events.lock().unwrap().clone();
    assert!(events.iter().any(|(name, _)| name == "galleria:state"));
    for (name, value) in events {
        for secret in secrets_of_the_seed() {
            assert!(
                !value.to_string().contains(&secret),
                "{name} carried a secret: {value}"
            );
        }
    }
    world.stop();
}

/// What the process wrote on its `stderr` while `work` ran.
fn stderr_during(work: impl FnOnce()) -> String {
    // `stderr` is one for the whole process: two captures at once would take each other's lines and put the wrong one back
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _only = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    use std::io::{Read, Seek, SeekFrom};
    let mut capture = tempfile::tempfile().unwrap();
    let saved = nix::unistd::dup(std::io::stderr()).unwrap();
    nix::unistd::dup2_stderr(&capture).unwrap();
    work();
    nix::unistd::dup2_stderr(&saved).unwrap();
    let mut text = String::new();
    capture.seek(SeekFrom::Start(0)).unwrap();
    capture.read_to_string(&mut text).unwrap();
    text
}

#[test]
fn the_capture_of_stderr_sees_what_is_written_to_it() {
    // the proof below means something only if the capture catches what a thread of the app writes
    let text = stderr_during(|| {
        std::thread::spawn(|| {
            nix::unistd::write(std::io::stderr(), b"probe-line").unwrap();
        })
        .join()
        .unwrap();
    });
    assert!(text.contains("probe-line"), "{text}");
}

#[test]
fn stderr_never_carries_connect_params() {
    let world = start(Options::default());
    let text = stderr_during(|| {
        // a connection that works, one that is refused by the daemon, one that cannot read its keys, one without a daemon
        world.connect("Homelab", false).unwrap();
        world.until_state("connected", None);
        world.connect("VPS Hetzner", false).unwrap_err();
        world.invoke("disconnect", json!({})).unwrap();
        world.until_state("disconnected", None);
        FileSecretStore::open(&world.secrets_dir())
            .unwrap()
            .delete_all(&world.id("VPS Hetzner"))
            .unwrap();
        world.connect("VPS Hetzner", false).unwrap_err();
    });
    for secret in secrets_of_the_seed() {
        assert!(!text.contains(&secret), "stderr carried a secret: {text}");
    }
    for word in ["private_key", "secret_blocks", "preshared", "tls-crypt"] {
        assert!(!text.contains(word), "stderr carried `{word}`: {text}");
    }
    world.stop();
    // and the daemon that is not there
    let absent = start(Options {
        daemon: false,
        ..Options::default()
    });
    let text = stderr_during(|| {
        absent.connect("Homelab", false).unwrap_err();
    });
    assert!(!text.contains(seed::SECRET_MARK), "{text}");
    let _ = Secret::new("");
}

/// A window «Save» that always says the same place.
struct SavesTo(std::path::PathBuf);

impl galleria_gui::export::SavePicker for SavesTo {
    fn pick(&self, _suggested_name: &str) -> Option<std::path::PathBuf> {
        Some(self.0.clone())
    }
}

#[test]
fn no_command_returns_a_secret_after_a_full_round() {
    // G7, the whole of B5: every command of the connection and every event, in a round that types the user, the password, a code and a
    // passphrase, remembers, forgets, reads the log and exports it, with the keys of the seed in the profiles and repeated by a backend
    // that cannot keep its mouth shut. Nothing the page can read, and nothing written for it, holds any of them.
    const TYPED: [&str; 4] = [
        "carla.v",
        "PW-5f3a-typed",
        "OTP-482913-typed",
        "PASSPHRASE-7c1e-typed",
    ];
    let known: Vec<&str> = TYPED.iter().copied().chain([seed::SECRET_MARK]).collect();
    let out = tempfile::tempdir().unwrap();
    let mut answers: Vec<(String, String)> = vec![];
    let mut events: Vec<(String, Value)> = vec![];
    let mut exported = String::new();
    // each scenario is a backend that asks (or leaks) differently; the round is the same
    for (scenario, profile, input) in [
        (
            "credentials",
            "Datacenter Milano",
            Some(json!({ "username": TYPED[0], "password": TYPED[1] })),
        ),
        ("otp", "Datacenter Milano", Some(json!({ "otp": TYPED[2] }))),
        (
            "passphrase",
            "Datacenter Milano",
            Some(json!({ "key_passphrase": TYPED[3] })),
        ),
        ("leaky", "Homelab", None),
    ] {
        let target = out.path().join(format!("{scenario}.txt"));
        let world = start(Options {
            scenario,
            picker: Some(Arc::new(SavesTo(target.clone()))),
            ..Options::default()
        });
        let mut note = |what: &str, answer: Result<Value, Value>| {
            answers.push((
                format!("{scenario}: {what}"),
                answer.unwrap_or_else(|e| e).to_string(),
            ));
        };
        world.phrases();
        note("connect_profile", world.connect(profile, false));
        if let Some(values) = input {
            world.until_state("needs_input", None);
            let (session, request) = {
                let events = world.events.lock().unwrap();
                let state = events
                    .iter()
                    .rev()
                    .find(|(name, value)| {
                        name == "galleria:state" && value["state"] == "needs_input"
                    })
                    .map(|(_, value)| value.clone())
                    .unwrap();
                (
                    state["session"].as_u64().unwrap(),
                    state["input_request"]["request"].as_u64().unwrap(),
                )
            };
            note(
                "provide_input",
                world.invoke("provide_input", json!({ "session": session, "request": request, "values": values, "remember": true })),
            );
        }
        world.until_state("connected", None);
        note(
            "logs_get",
            world.invoke("logs_get", json!({ "after": null, "limit": 1000 })),
        );
        note("logs_export", world.invoke("logs_export", json!({})));
        note(
            "secret_store_status",
            world.invoke("secret_store_status", json!({})),
        );
        note(
            "profile_get",
            world.invoke("profile_get", json!({ "id": world.id(profile) })),
        );
        note(
            "forget_credentials",
            world.invoke("forget_credentials", json!({ "id": world.id(profile) })),
        );
        note("daemon_view", world.invoke("daemon_view", json!({})));
        note("disconnect", world.invoke("disconnect", json!({})));
        world.until_state("disconnected", None);
        events.extend(world.events.lock().unwrap().iter().cloned());
        exported.push_str(&fs::read_to_string(&target).unwrap());
        world.stop();
    }
    // the round happened: the log was read and exported four times, with something in it, and the events include the states
    assert!(answers.len() >= 30, "{} answers", answers.len());
    assert!(
        exported.contains("handshake with key [redatto] done"),
        "the leaky backend's line is in the export, redacted: {exported}"
    );
    assert!(events.iter().any(|(name, _)| name == "galleria:state"));
    for secret in known {
        for (what, text) in &answers {
            assert!(!text.contains(secret), "{what} returned `{secret}`: {text}");
        }
        for (name, value) in &events {
            assert!(
                !value.to_string().contains(secret),
                "{name} carried `{secret}`: {value}"
            );
        }
        assert!(
            !exported.contains(secret),
            "the exported log holds `{secret}`"
        );
    }
}
