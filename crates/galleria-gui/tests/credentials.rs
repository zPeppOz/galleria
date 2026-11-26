//! Asking for credentials from the window and remembering them (plan B5, T6): `provide_input` and `forget_credentials` against a real
//! daemon with a fake backend, «Ricorda» only after `connected`, never for a code or a passphrase, and that no command, no event
//! and no line of `stderr` carries what was typed (the rule of «Il viaggio di una credenziale»).
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;
mod support;
#[path = "support/world.rs"]
mod world;

use galleria_gui::session::Sessions;
use galleria_profile::{FileSecretStore, ProfileStore, SecretStore};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::Manager;
use uuid::Uuid;
use world::*;

const USER: &str = "carla.v";
const PASSWORD: &str = "PW-5f3a-typed";
const OTP: &str = "OTP-482913-typed";
const PASSPHRASE: &str = "PASSPHRASE-7c1e-typed";

fn begin(scenario: &'static str) -> World {
    start(Options {
        scenario,
        ..Options::default()
    })
}

/// The `session` and the `request` of the question the daemon asked last, once it has asked.
fn question(world: &World) -> (u64, u64) {
    world.until_state("needs_input", None);
    let events = world.events.lock().unwrap();
    let state = events
        .iter()
        .rev()
        .find(|(name, value)| name == "galleria:state" && value["state"] == "needs_input")
        .map(|(_, value)| value.clone())
        .unwrap();
    (
        state["session"].as_u64().unwrap(),
        state["input_request"]["request"].as_u64().unwrap(),
    )
}

fn answer(
    world: &World,
    (session, request): (u64, u64),
    values: Value,
    remember: bool,
) -> Result<Value, Value> {
    world.invoke(
        "provide_input",
        json!({ "session": session, "request": request, "values": values, "remember": remember }),
    )
}

fn credentials() -> Value {
    json!({ "username": USER, "password": PASSWORD })
}

fn sessions(world: &World) -> Arc<Sessions> {
    Arc::clone(&*world._app.state::<Arc<Sessions>>())
}

fn store(world: &World) -> FileSecretStore {
    FileSecretStore::open(&world.secrets_dir()).unwrap()
}

fn kept(world: &World, name: &str, key: &str) -> Option<String> {
    store(world)
        .get(&world.id(name), key)
        .unwrap()
        .map(|secret| secret.expose().to_owned())
}

fn refs(world: &World, name: &str) -> Vec<String> {
    ProfileStore::open(&world.secrets_dir())
        .unwrap()
        .find(&world.id(name).to_string())
        .unwrap()
        .secret_refs
}

fn names(world: &World, name: &str) -> (bool, bool) {
    let refs = refs(world, name);
    (
        refs.iter().any(|r| r == "openvpn.username"),
        refs.iter().any(|r| r == "openvpn.password"),
    )
}

fn until(what: &str, mut condition: impl FnMut() -> bool) {
    let begun = Instant::now();
    while !condition() {
        assert!(begun.elapsed() < WAIT, "never happened: {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

const DC: &str = "Datacenter Milano";

#[test]
fn provide_input_reaches_the_daemon_and_nothing_else() {
    let world = begin("credentials");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    assert_eq!(
        answer(&world, asked, credentials(), false).unwrap(),
        Value::Null
    );
    world.until_state("connected", None);
    let inputs = world.daemon().harness.inputs.lock().unwrap().clone();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["username"], USER);
    assert_eq!(inputs[0]["password"], PASSWORD);
    // asked not to remember: nothing is kept, and nothing the page could read repeats the values
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    assert_eq!(names(&world, DC), (false, false));
    assert!(!sessions(&world).has_pending());
    for (name, value) in world.events.lock().unwrap().iter() {
        let text = value.to_string();
        assert!(
            !text.contains(PASSWORD) && !text.contains(USER),
            "{name}: {text}"
        );
    }
    world.stop();
}

#[test]
fn remember_saves_only_after_connected() {
    let world = begin("stuck");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    answer(&world, asked, credentials(), true).unwrap();
    // answered and not connected yet: held in memory by the session, in the archive nowhere
    assert!(sessions(&world).has_pending());
    assert_eq!(kept(&world, DC, "openvpn.username"), None);
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    assert_eq!(names(&world, DC), (false, false));
    world.stop();

    let world = begin("credentials");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    answer(&world, asked, credentials(), true).unwrap();
    world.until_state("connected", None);
    assert_eq!(kept(&world, DC, "openvpn.username").as_deref(), Some(USER));
    assert_eq!(
        kept(&world, DC, "openvpn.password").as_deref(),
        Some(PASSWORD)
    );
    assert_eq!(names(&world, DC), (true, true));
    assert!(!sessions(&world).has_pending(), "what was kept is dropped");
    // the page is told the names and never the values
    let detail = world
        .invoke("profile_get", json!({ "id": world.id(DC) }))
        .unwrap();
    assert!(
        detail["secret_refs"]
            .to_string()
            .contains("openvpn.password")
    );
    assert!(!detail.to_string().contains(PASSWORD));
    assert!(!detail.to_string().contains(USER));
    world.stop();
}

#[test]
fn a_failed_login_saves_nothing() {
    let world = begin("auth-failed");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    answer(&world, asked, credentials(), true).unwrap();
    world.until_state("error", None);
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    assert_eq!(kept(&world, DC, "openvpn.username"), None);
    assert_eq!(names(&world, DC), (false, false));
    assert!(!sessions(&world).has_pending());
    world.stop();
}

#[test]
fn a_disconnect_before_connected_saves_nothing() {
    let world = begin("stuck");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    answer(&world, asked, credentials(), true).unwrap();
    assert!(sessions(&world).has_pending());
    world.invoke("disconnect", json!({})).unwrap();
    world.until_state("disconnected", None);
    until("the pending credentials were dropped", || {
        !sessions(&world).has_pending()
    });
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    assert_eq!(names(&world, DC), (false, false));
    world.stop();
}

#[test]
fn otp_and_passphrase_are_never_kept() {
    for (scenario, values) in [
        ("otp", json!({ "otp": OTP })),
        ("passphrase", json!({ "key_passphrase": PASSPHRASE })),
    ] {
        let world = begin(scenario);
        world.connect(DC, false).unwrap();
        let asked = question(&world);
        // asked to remember, and not for a request of credentials: nothing is held
        answer(&world, asked, values, true).unwrap();
        assert!(!sessions(&world).has_pending(), "{scenario}");
        world.until_state("connected", None);
        for key in [
            "openvpn.username",
            "openvpn.password",
            "openvpn.otp",
            "openvpn.key_passphrase",
        ] {
            assert_eq!(kept(&world, DC, key), None, "{scenario}: {key}");
        }
        assert_eq!(names(&world, DC), (false, false));
        assert!(
            !std::fs::read_to_string(
                world
                    .secrets_dir()
                    .join("secrets")
                    .join(format!("{}.toml", world.id(DC)))
            )
            .unwrap_or_default()
            .contains("typed")
        );
        world.stop();
    }
}

#[test]
fn the_next_connection_sends_the_remembered_credentials() {
    let world = begin("credentials");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    answer(&world, asked, credentials(), true).unwrap();
    world.until_state("connected", None);
    world.invoke("disconnect", json!({})).unwrap();
    world.until_state("disconnected", None);
    world.connect(DC, false).unwrap();
    until("the second connection", || {
        world.daemon().harness.connects.lock().unwrap().len() == 2
    });
    world.until_state("connected", None);
    let sent = world.daemon().harness.connects.lock().unwrap()[1].clone();
    assert_eq!(sent["backend"]["username"], USER);
    assert_eq!(sent["backend"]["password"], PASSWORD);
    let asks = world
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|(name, value)| name == "galleria:state" && value["state"] == "needs_input")
        .count();
    let questions: Vec<_> = world
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|(name, value)| name == "galleria:state" && value["state"] == "needs_input")
        .map(|(_, value)| value["session"].clone())
        .collect();
    assert!(
        questions.iter().all(|s| *s == questions[0]),
        "only the first session asked ({asks} events)"
    );
    world.stop();
}

#[test]
fn a_remembered_credential_gone_from_the_store_is_not_an_error() {
    let world = begin("credentials");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    answer(&world, asked, credentials(), true).unwrap();
    world.until_state("connected", None);
    world.invoke("disconnect", json!({})).unwrap();
    world.until_state("disconnected", None);
    // taken out by hand, as someone who cleans the keyring would
    store(&world)
        .delete(&world.id(DC), "openvpn.password")
        .unwrap();
    world.connect(DC, false).unwrap();
    until("the second connection", || {
        world.daemon().harness.connects.lock().unwrap().len() == 2
    });
    let sent = world.daemon().harness.connects.lock().unwrap()[1].clone();
    assert!(sent["backend"]["password"].is_null(), "{sent}");
    assert!(sent["backend"]["username"].is_null(), "{sent}");
    // the profile does not claim credentials it no longer has
    assert_eq!(names(&world, DC), (false, false));
    world.stop();
}

#[test]
fn forget_removes_the_two_keys_and_their_names() {
    let world = begin("credentials");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    answer(&world, asked, credentials(), true).unwrap();
    world.until_state("connected", None);
    let block = kept(&world, DC, "openvpn.block.tls-crypt");
    assert!(block.is_some());
    assert_eq!(
        world
            .invoke("forget_credentials", json!({ "id": world.id(DC) }))
            .unwrap(),
        Value::Null
    );
    assert_eq!(kept(&world, DC, "openvpn.username"), None);
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    assert_eq!(names(&world, DC), (false, false));
    // the keys of the profile are not credentials
    assert_eq!(kept(&world, DC, "openvpn.block.tls-crypt"), block);
    assert!(
        refs(&world, DC)
            .iter()
            .any(|r| r == "openvpn.block.tls-crypt")
    );
    // forgetting what is not there is not an error
    world
        .invoke("forget_credentials", json!({ "id": world.id(DC) }))
        .unwrap();
    let missing = world
        .invoke("forget_credentials", json!({ "id": Uuid::new_v4() }))
        .unwrap_err();
    assert_eq!(code(&missing), "gui_profile_not_found");
    world.stop();
}

#[test]
fn an_expired_request_is_input_not_expected_and_changes_nothing() {
    let world = begin("credentials");
    world.connect(DC, false).unwrap();
    let (session, request) = question(&world);
    let late = answer(&world, (session, request + 7), credentials(), true).unwrap_err();
    assert_eq!(code(&late), "input_not_expected");
    assert!(!sessions(&world).has_pending());
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    // the question is still there and the right answer still works
    answer(&world, (session, request), credentials(), false).unwrap();
    world.until_state("connected", None);
    world.stop();
}

#[test]
fn pending_is_dropped_when_another_session_starts() {
    let world = begin("stuck");
    world.connect(DC, false).unwrap();
    let asked = question(&world);
    answer(&world, asked, credentials(), true).unwrap();
    assert!(sessions(&world).has_pending());
    world.connect("Ufficio Napoli", true).unwrap();
    until("the credentials of the first session were dropped", || {
        !sessions(&world).has_pending()
    });
    assert_eq!(kept(&world, DC, "openvpn.password"), None);
    assert_eq!(names(&world, DC), (false, false));
    world.stop();
}

#[test]
fn no_command_returns_a_secret() {
    // a round with a password that is remembered, a code and a passphrase
    let typed = [PASSWORD, OTP, PASSPHRASE, USER];
    let mut seen: Vec<(String, String)> = vec![];
    let mut events: Vec<(String, Value)> = vec![];
    for (scenario, values, remember) in [
        ("credentials", credentials(), true),
        ("otp", json!({ "otp": OTP }), true),
        ("passphrase", json!({ "key_passphrase": PASSPHRASE }), true),
    ] {
        let world = begin(scenario);
        let mut note = |what: &str, answer: Result<Value, Value>| {
            seen.push((
                format!("{scenario}: {what}"),
                answer.unwrap_or_else(|e| e).to_string(),
            ));
        };
        note("connect_profile", world.connect(DC, false));
        let asked = question(&world);
        note(
            "provide_input",
            answer(&world, asked, values.clone(), remember),
        );
        note(
            "provide_input again",
            answer(&world, asked, values, remember),
        );
        world.until_state("connected", None);
        note("daemon_view", world.invoke("daemon_view", json!({})));
        note(
            "profile_get",
            world.invoke("profile_get", json!({ "id": world.id(DC) })),
        );
        note("profiles_list", world.invoke("profiles_list", json!({})));
        note(
            "secret_store_status",
            world.invoke("secret_store_status", json!({})),
        );
        note(
            "forget_credentials",
            world.invoke("forget_credentials", json!({ "id": world.id(DC) })),
        );
        note("disconnect", world.invoke("disconnect", json!({})));
        world.until_state("disconnected", None);
        events.extend(world.events.lock().unwrap().iter().cloned());
        world.stop();
    }
    assert!(seen.len() > 20);
    for (what, text) in &seen {
        for secret in typed {
            assert!(!text.contains(secret), "{what} returned `{secret}`: {text}");
        }
    }
    // the events are the other way the page learns anything
    assert!(events.iter().any(|(name, _)| name == "galleria:state"));
    for (name, value) in &events {
        for secret in typed {
            assert!(
                !value.to_string().contains(secret),
                "{name} carried `{secret}`: {value}"
            );
        }
    }
}

#[test]
fn no_debug_text_carries_what_was_typed() {
    use galleria_ipc::{InputValues, ProvideInputParams, Secret};
    let params = ProvideInputParams {
        session: 1,
        request: 1,
        values: InputValues {
            username: None,
            password: Some(Secret::new(PASSWORD)),
            otp: Some(Secret::new(OTP)),
            key_passphrase: Some(Secret::new(PASSPHRASE)),
        },
    };
    let text = format!("{params:?}");
    for secret in [PASSWORD, OTP, PASSPHRASE] {
        assert!(!text.contains(secret), "{text}");
    }
}
