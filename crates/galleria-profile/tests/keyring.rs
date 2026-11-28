#![cfg(target_os = "linux")]
//! The keyring store against a real Secret Service (plan B5, T2). Every test here is `#[ignore]`: `cargo test --workspace`
//! does not run them, and `scripts/gui.sh keyring` (lab/tests/gui-keyring.sh) runs them three times, each under a session
//! bus of its own with GNOME Keyring unlocked, locked or absent, and says which with `GALLERIA_KEYRING_CASE`.
//!
//! A test that finds the wrong case, or none, **fails** and says what is missing: nothing here skips itself, so a run
//! without the bus cannot come out green (and `scripts/check-gates.py` wants every name with `ok` in the log).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::Secret;
use galleria_profile::{
    FileSecretStore, KeyringSecretStore, LayeredSecretStore, SecretStore, StoreError, StoreKind,
    Writes, keyring_present,
};
use std::{
    os::unix::fs::PermissionsExt,
    process::Command,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// The profile of the item that gui-keyring.sh stores before it locks the keyring (the locked round).
const SEEDED_PROFILE: &str = "5ee0ed00-0000-4000-8000-000000000001";
const SEEDED_KEY: &str = "wireguard.private_key";

fn must_be(wanted: &str) {
    let got = std::env::var("GALLERIA_KEYRING_CASE").unwrap_or_default();
    assert_eq!(
        got, wanted,
        "this test needs GALLERIA_KEYRING_CASE={wanted}, set by scripts/gui.sh keyring (got {got:?})"
    );
}

fn store() -> KeyringSecretStore {
    KeyringSecretStore::open()
        .expect(
            "no Secret Service on the session bus: this test runs through scripts/gui.sh keyring, \
             which starts GNOME Keyring",
        )
        .with_prompt_timeout(3)
}

fn secret_tool(args: &[&str]) -> String {
    let out = Command::new("secret-tool")
        .args(args)
        .output()
        .expect("secret-tool is not installed: it comes with libsecret-tools in the image of scripts/gui.sh");
    assert!(
        out.status.success(),
        "secret-tool {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn text(store: &dyn SecretStore, id: &Uuid, key: &str) -> Option<String> {
    store.get(id, key).unwrap().map(|s| s.expose().to_string())
}

#[test]
#[ignore = "needs GNOME Keyring: scripts/gui.sh keyring"]
fn keyring_roundtrip() {
    must_be("unlocked");
    let store = store();
    let id = Uuid::new_v4();
    assert_eq!(text(&store, &id, "k"), None);
    store.set(&id, "k", &Secret::new("first")).unwrap();
    assert_eq!(text(&store, &id, "k").as_deref(), Some("first"));
    store.set(&id, "k", &Secret::new("second")).unwrap();
    assert_eq!(
        text(&store, &id, "k").as_deref(),
        Some("second"),
        "a second set replaces the value"
    );
    assert_eq!(
        items_of(&id).matches("attribute.key = k").count(),
        1,
        "and does not add a second item"
    );
    assert_eq!(text(&store, &id, "other"), None);
    store.delete_all(&id).unwrap();
}

#[test]
#[ignore = "needs GNOME Keyring: scripts/gui.sh keyring"]
fn keyring_items_carry_the_three_attributes_and_the_label() {
    must_be("unlocked");
    let id = Uuid::new_v4();
    let named = store().labelled("Ufficio Napoli");
    named
        .set(&id, "openvpn.block.key", &Secret::new("SEED-SECRET-LABEL"))
        .unwrap();
    // read back the way another program would
    let found = items_of(&id);
    for line in [
        "attribute.service = galleria".to_string(),
        format!("attribute.profile = {id}"),
        "attribute.key = openvpn.block.key".to_string(),
        "label = Galleria: Ufficio Napoli (openvpn.block.key)".to_string(),
    ] {
        assert!(found.contains(&line), "{line:?} is not in:\n{found}");
    }
    assert_eq!(
        secret_tool(&[
            "lookup",
            "service",
            "galleria",
            "profile",
            &id.to_string(),
            "key",
            "openvpn.block.key"
        ]),
        "SEED-SECRET-LABEL"
    );
    // without a name the label has the id
    let unnamed = Uuid::new_v4();
    store().set(&unnamed, "k", &Secret::new("v")).unwrap();
    let found = items_of(&unnamed);
    assert!(
        found.contains(&format!("label = Galleria: {unnamed} (k)")),
        "{found}"
    );
    named.delete_all(&id).unwrap();
    named.delete_all(&unnamed).unwrap();
}

#[test]
#[ignore = "needs GNOME Keyring: scripts/gui.sh keyring"]
fn keyring_delete_one_and_all() {
    must_be("unlocked");
    let store = store();
    let id = Uuid::new_v4();
    for key in ["a", "b", "c"] {
        store.set(&id, key, &Secret::new(key)).unwrap();
    }
    store.delete(&id, "a").unwrap();
    assert_eq!(text(&store, &id, "a"), None);
    assert_eq!(text(&store, &id, "b").as_deref(), Some("b"));
    store.delete(&id, "a").unwrap(); // a key that is not there is fine
    store.delete_all(&id).unwrap();
    for key in ["a", "b", "c"] {
        assert_eq!(text(&store, &id, key), None, "{key}");
    }
    let left = items_of(&id);
    assert!(!left.contains("attribute.key"), "an item is left: {left}");
    store.delete_all(&id).unwrap(); // nothing at all is fine too
}

#[test]
#[ignore = "needs GNOME Keyring: scripts/gui.sh keyring"]
fn keyring_two_profiles_do_not_mix() {
    must_be("unlocked");
    let store = store();
    let (one, two) = (Uuid::new_v4(), Uuid::new_v4());
    store.set(&one, "k", &Secret::new("of-one")).unwrap();
    store.set(&two, "k", &Secret::new("of-two")).unwrap();
    assert_eq!(text(&store, &one, "k").as_deref(), Some("of-one"));
    assert_eq!(text(&store, &two, "k").as_deref(), Some("of-two"));
    store.delete_all(&one).unwrap();
    assert_eq!(text(&store, &one, "k"), None);
    assert_eq!(text(&store, &two, "k").as_deref(), Some("of-two"));
    store.delete_all(&two).unwrap();
}

#[test]
#[ignore = "needs GNOME Keyring: scripts/gui.sh keyring"]
fn keyring_value_with_newlines_survives() {
    must_be("unlocked");
    let store = store();
    let id = Uuid::new_v4();
    let pem = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n\nàèù ünïcode ✓\n-----END PRIVATE KEY-----\n";
    store
        .set(&id, "openvpn.block.key", &Secret::new(pem))
        .unwrap();
    assert_eq!(text(&store, &id, "openvpn.block.key").as_deref(), Some(pem));
    // the empty string is a value too
    store.set(&id, "empty", &Secret::new("")).unwrap();
    assert_eq!(text(&store, &id, "empty").as_deref(), Some(""));
    store.delete_all(&id).unwrap();
}

#[test]
#[ignore = "needs GNOME Keyring locked: scripts/gui.sh keyring"]
fn keyring_locked_is_reported_within_five_seconds() {
    must_be("locked");
    let store = store();
    let seeded: Uuid = SEEDED_PROFILE.parse().unwrap();
    let new = Uuid::new_v4();
    let started = Instant::now();
    // an item that is there, a key that is not, a write, a delete: a locked keyring answers all of them the same way
    assert!(
        matches!(
            store.get(&seeded, SEEDED_KEY),
            Err(StoreError::KeyringLocked)
        ),
        "get of an item that is there"
    );
    assert!(
        matches!(store.get(&new, "k"), Err(StoreError::KeyringLocked)),
        "get of an item that is not there"
    );
    assert!(
        matches!(
            store.set(&new, "k", &Secret::new("SEED-SECRET-LOCKED")),
            Err(StoreError::KeyringLocked)
        ),
        "set"
    );
    assert!(
        matches!(
            store.delete(&seeded, SEEDED_KEY),
            Err(StoreError::KeyringLocked)
        ),
        "delete"
    );
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(5 * 4),
        "four calls took {took:?}: each one must answer in under five seconds"
    );
    // nothing reached the keyring while it was locked
    let found = Command::new("secret-tool")
        .args(["search", "--all", "profile", &new.to_string()])
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&found.stdout).contains("attribute.key"),
        "an item was written into a locked keyring"
    );
}

#[test]
#[ignore = "needs GNOME Keyring locked: scripts/gui.sh keyring"]
fn keyring_present_does_not_unlock() {
    must_be("locked");
    assert!(keyring_present(), "the Secret Service is on the bus");
    assert!(KeyringSecretStore::open().is_some());
    let locked = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "-d",
            "org.freedesktop.secrets",
            "-o",
            "/org/freedesktop/secrets/collection/login",
            "-m",
            "org.freedesktop.DBus.Properties.Get",
            "org.freedesktop.Secret.Collection",
            "Locked",
        ])
        .output()
        .expect(
            "gdbus is not installed: it comes with libglib2.0-bin in the image of scripts/gui.sh",
        );
    let answer = String::from_utf8_lossy(&locked.stdout).to_string();
    assert!(
        answer.contains("true"),
        "the collection is not locked any more after asking if the keyring is there: {answer:?} {}",
        String::from_utf8_lossy(&locked.stderr)
    );
}

#[test]
#[ignore = "needs a session bus without a Secret Service: scripts/gui.sh keyring"]
fn keyring_absent_opens_as_none() {
    must_be("absent");
    assert!(
        std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok_and(|a| !a.is_empty()),
        "this case is a bus without the service, not no bus at all: DBUS_SESSION_BUS_ADDRESS is not set"
    );
    let started = Instant::now();
    assert!(!keyring_present());
    assert!(KeyringSecretStore::open().is_none());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "a bus without the service must answer quickly, took {:?}",
        started.elapsed()
    );
}

#[test]
#[ignore = "needs GNOME Keyring without a default collection: scripts/gui.sh keyring"]
fn keyring_without_a_default_collection_is_no_keyring() {
    must_be("nodefault");
    // Ubuntu 26.04 with an automatic login: the Secret Service answers on the bus, but nothing was ever made to keep a secret in.
    // The app must treat it as it treats a bus without the service (D25): the secrets go to the file, with the warning of the
    // window, and the import is not stopped by «The keyring did not answer».
    assert!(
        !keyring_present(),
        "a service with no default collection is not a keyring"
    );
    assert!(KeyringSecretStore::open().is_none());
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let layered = LayeredSecretStore::new(
        KeyringSecretStore::open().map(|k| Box::new(k) as Box<dyn SecretStore>),
        Box::new(FileSecretStore::open(dir.path()).unwrap()),
        Writes::Keyring,
    );
    assert_eq!(
        layered.kind(),
        StoreKind::File,
        "the warning is shown for this"
    );
    let id = Uuid::new_v4();
    layered.set(&id, "k", &Secret::new("in the file")).unwrap();
    assert_eq!(text(&layered, &id, "k").as_deref(), Some("in the file"));
    assert!(
        dir.path()
            .join("secrets")
            .join(format!("{id}.toml"))
            .is_file(),
        "the secret is in the file"
    );
}

/// What `secret-tool search` says about the items of a profile, attributes included (it prints those on stderr, and
/// exits with 1 when there is none).
fn items_of(id: &Uuid) -> String {
    let out = Command::new("secret-tool")
        .args(["search", "--all", "profile", &id.to_string()])
        .output()
        .expect("secret-tool is not installed: it comes with libsecret-tools in the image of scripts/gui.sh");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
