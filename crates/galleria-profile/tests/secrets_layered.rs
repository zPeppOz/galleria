//! The layered archive of the secrets (plan B5, T1): the keyring first, then the file. The keyring here is an archive in
//! memory with a «locked» switch, so these tests run without a bus; the real one is tried by `tests/keyring.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::Secret;
use galleria_profile::*;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Default)]
struct Memory {
    values: BTreeMap<(Uuid, String), String>,
    locked: bool,
    broken: bool,
}

/// A keyring in memory; the clone given to the test keeps looking at the same values.
#[derive(Clone, Default)]
struct FakeKeyring(Arc<Mutex<Memory>>);

impl FakeKeyring {
    fn lock(&self, locked: bool) {
        self.0.lock().unwrap().locked = locked;
    }
    fn break_it(&self) {
        self.0.lock().unwrap().broken = true;
    }
    fn holds(&self, id: &Uuid, key: &str) -> Option<String> {
        self.0
            .lock()
            .unwrap()
            .values
            .get(&(*id, key.into()))
            .cloned()
    }
    fn check(m: &Memory) -> Result<(), StoreError> {
        if m.locked {
            return Err(StoreError::KeyringLocked);
        }
        if m.broken {
            return Err(StoreError::Keyring("the service went away".into()));
        }
        Ok(())
    }
}

impl SecretStore for FakeKeyring {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError> {
        let m = self.0.lock().unwrap();
        Self::check(&m)?;
        Ok(m.values.get(&(*id, key.into())).map(Secret::new))
    }
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError> {
        let mut m = self.0.lock().unwrap();
        Self::check(&m)?;
        m.values.insert((*id, key.into()), value.expose().into());
        Ok(())
    }
    fn delete(&self, id: &Uuid, key: &str) -> Result<(), StoreError> {
        let mut m = self.0.lock().unwrap();
        Self::check(&m)?;
        m.values.remove(&(*id, key.into()));
        Ok(())
    }
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError> {
        let mut m = self.0.lock().unwrap();
        Self::check(&m)?;
        m.values.retain(|(i, _), _| i != id);
        Ok(())
    }
}

fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

struct World {
    dir: tempfile::TempDir,
    keyring: FakeKeyring,
}

impl World {
    fn new() -> Self {
        Self {
            dir: private_dir(),
            keyring: FakeKeyring::default(),
        }
    }
    fn file(&self) -> FileSecretStore {
        FileSecretStore::open(self.dir.path()).unwrap()
    }
    fn layered(&self, writes: Writes) -> LayeredSecretStore {
        LayeredSecretStore::new(
            Some(Box::new(self.keyring.clone())),
            Box::new(self.file()),
            writes,
        )
    }
    fn without_keyring(&self, writes: Writes) -> LayeredSecretStore {
        LayeredSecretStore::new(None, Box::new(self.file()), writes)
    }
}

fn read(store: &dyn SecretStore, id: &Uuid, key: &str) -> Option<String> {
    store.get(id, key).unwrap().map(|s| s.expose().to_string())
}

/// The bytes of every file under `secrets/`, by name.
fn secrets_folder(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir.join("secrets")).unwrap() {
        let entry = entry.unwrap();
        out.insert(
            entry.file_name().to_string_lossy().into_owned(),
            fs::read(entry.path()).unwrap(),
        );
    }
    out
}

#[test]
fn layered_reads_the_keyring_first_then_the_file() {
    let w = World::new();
    let id = Uuid::new_v4();
    w.keyring
        .set(&id, "both", &Secret::new("from-keyring"))
        .unwrap();
    w.file()
        .set(&id, "both", &Secret::new("from-file"))
        .unwrap();
    w.file()
        .set(&id, "only-file", &Secret::new("file-only"))
        .unwrap();
    for writes in [Writes::Keyring, Writes::File] {
        let layered = w.layered(writes);
        assert_eq!(read(&layered, &id, "both").as_deref(), Some("from-keyring"));
        assert_eq!(
            read(&layered, &id, "only-file").as_deref(),
            Some("file-only")
        );
        assert_eq!(read(&layered, &id, "nowhere"), None);
    }
}

#[test]
fn layered_gui_writes_go_to_the_keyring_and_the_file_copy_is_removed() {
    let w = World::new();
    let id = Uuid::new_v4();
    w.file().set(&id, "key", &Secret::new("old")).unwrap();
    w.file().set(&id, "other", &Secret::new("kept")).unwrap();
    let layered = w.layered(Writes::Keyring);
    layered.set(&id, "key", &Secret::new("new")).unwrap();
    assert_eq!(w.keyring.holds(&id, "key").as_deref(), Some("new"));
    assert_eq!(
        read(&w.file(), &id, "key"),
        None,
        "the copy in the file is gone"
    );
    assert_eq!(read(&w.file(), &id, "other").as_deref(), Some("kept"));
    assert_eq!(read(&layered, &id, "key").as_deref(), Some("new"));
}

#[test]
fn after_a_keyring_write_the_file_copy_is_gone() {
    // the file keeps no second value even when it was the only place that had the key before
    let w = World::new();
    let id = Uuid::new_v4();
    w.file().set(&id, "only", &Secret::new("one")).unwrap();
    w.layered(Writes::Keyring)
        .set(&id, "only", &Secret::new("two"))
        .unwrap();
    assert_eq!(read(&w.file(), &id, "only"), None);
    // a file copy that cannot be removed does not undo the write: the read gives the new value
    let w = World::new();
    w.file().set(&id, "k", &Secret::new("old")).unwrap();
    let layered = w.layered(Writes::Keyring);
    fs::set_permissions(
        w.dir.path().join("secrets"),
        fs::Permissions::from_mode(0o500),
    )
    .unwrap();
    layered.set(&id, "k", &Secret::new("new")).unwrap();
    fs::set_permissions(
        w.dir.path().join("secrets"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert_eq!(read(&layered, &id, "k").as_deref(), Some("new"));
}

#[test]
fn layered_without_a_keyring_writes_the_file() {
    let w = World::new();
    let id = Uuid::new_v4();
    let layered = w.without_keyring(Writes::Keyring);
    layered.set(&id, "key", &Secret::new("v")).unwrap();
    assert_eq!(read(&w.file(), &id, "key").as_deref(), Some("v"));
    assert_eq!(read(&layered, &id, "key").as_deref(), Some("v"));
}

#[test]
fn kind_says_where_a_write_goes() {
    let w = World::new();
    assert_eq!(w.layered(Writes::Keyring).kind(), StoreKind::Keyring);
    assert_eq!(w.layered(Writes::File).kind(), StoreKind::File);
    assert_eq!(w.without_keyring(Writes::Keyring).kind(), StoreKind::File);
    assert_eq!(w.without_keyring(Writes::File).kind(), StoreKind::File);
}

#[test]
fn a_locked_keyring_is_an_error_and_writes_no_file() {
    let w = World::new();
    let id = Uuid::new_v4();
    // a key that lives in the file only: a locked keyring must not send the read there
    w.file().set(&id, "in-file", &Secret::new("f")).unwrap();
    let before = secrets_folder(w.dir.path());
    w.keyring.lock(true);
    let layered = w.layered(Writes::Keyring);
    assert!(matches!(
        layered.get(&id, "in-file"),
        Err(StoreError::KeyringLocked)
    ));
    assert!(matches!(
        layered.set(&id, "new", &Secret::new("n")),
        Err(StoreError::KeyringLocked)
    ));
    assert_eq!(secrets_folder(w.dir.path()), before, "same bytes as before");
    // locked is not absent: with a fresh folder nothing is created either
    let w = World::new();
    w.keyring.lock(true);
    assert!(matches!(
        w.layered(Writes::Keyring).set(&id, "k", &Secret::new("v")),
        Err(StoreError::KeyringLocked)
    ));
    assert!(secrets_folder(w.dir.path()).is_empty());
}

#[test]
fn a_keyring_that_fails_is_an_error_not_a_silent_fallback() {
    let w = World::new();
    let id = Uuid::new_v4();
    w.file().set(&id, "k", &Secret::new("f")).unwrap();
    w.keyring.break_it();
    let layered = w.layered(Writes::Keyring);
    assert!(matches!(layered.get(&id, "k"), Err(StoreError::Keyring(_))));
    assert!(matches!(
        layered.set(&id, "k", &Secret::new("x")),
        Err(StoreError::Keyring(_))
    ));
    assert_eq!(read(&w.file(), &id, "k").as_deref(), Some("f"));
}

#[test]
fn layered_cli_writes_stay_in_the_file() {
    let w = World::new();
    let id = Uuid::new_v4();
    let layered = w.layered(Writes::File);
    layered.set(&id, "key", &Secret::new("v")).unwrap();
    assert_eq!(read(&w.file(), &id, "key").as_deref(), Some("v"));
    assert_eq!(w.keyring.holds(&id, "key"), None);
    // and a locked keyring does not stop a write that does not touch it
    w.keyring.lock(true);
    layered.set(&id, "key2", &Secret::new("v2")).unwrap();
    assert_eq!(read(&w.file(), &id, "key2").as_deref(), Some("v2"));
}

#[test]
fn delete_removes_one_key_from_both_and_keeps_the_others() {
    let w = World::new();
    let id = Uuid::new_v4();
    for key in ["a", "b"] {
        w.keyring.set(&id, key, &Secret::new("k")).unwrap();
        w.file().set(&id, key, &Secret::new("f")).unwrap();
    }
    for writes in [Writes::Keyring, Writes::File] {
        let layered = w.layered(writes);
        layered.delete(&id, "a").unwrap();
        assert_eq!(w.keyring.holds(&id, "a"), None);
        assert_eq!(read(&w.file(), &id, "a"), None);
        assert_eq!(w.keyring.holds(&id, "b").as_deref(), Some("k"));
        assert_eq!(read(&w.file(), &id, "b").as_deref(), Some("f"));
        // for the next round
        w.keyring.set(&id, "a", &Secret::new("k")).unwrap();
        w.file().set(&id, "a", &Secret::new("f")).unwrap();
    }
}

#[test]
fn delete_all_empties_both() {
    let w = World::new();
    let (id, other) = (Uuid::new_v4(), Uuid::new_v4());
    for who in [id, other] {
        w.keyring.set(&who, "a", &Secret::new("k")).unwrap();
        w.file().set(&who, "b", &Secret::new("f")).unwrap();
    }
    w.layered(Writes::Keyring).delete_all(&id).unwrap();
    assert_eq!(w.keyring.holds(&id, "a"), None);
    assert_eq!(read(&w.file(), &id, "b"), None);
    assert_eq!(w.keyring.holds(&other, "a").as_deref(), Some("k"));
    assert_eq!(read(&w.file(), &other, "b").as_deref(), Some("f"));
}

#[test]
fn deleting_a_key_that_is_not_there_is_fine() {
    let w = World::new();
    let id = Uuid::new_v4();
    w.layered(Writes::Keyring).delete(&id, "nothing").unwrap();
    w.without_keyring(Writes::File)
        .delete(&id, "nothing")
        .unwrap();
    w.layered(Writes::Keyring).delete_all(&id).unwrap();
    w.file().delete(&id, "nothing").unwrap();
}

#[test]
fn deleting_from_a_locked_keyring_is_reported_and_the_file_is_still_cleaned() {
    let w = World::new();
    let id = Uuid::new_v4();
    w.file().set(&id, "k", &Secret::new("f")).unwrap();
    w.keyring.lock(true);
    assert!(matches!(
        w.layered(Writes::Keyring).delete(&id, "k"),
        Err(StoreError::KeyringLocked)
    ));
    assert_eq!(
        read(&w.file(), &id, "k"),
        None,
        "the file side is done anyway"
    );
}

const EXAMPLE: &str = r#"
schema_version = 1
id = "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11"
name = "Ufficio Napoli"
protocol = "openvpn"
group = "Lavoro"
tags = []
favorite = false
notes = ""
created = "2026-11-10T10:00:00Z"
modified = "2026-11-10T10:00:00Z"
secret_refs = ["openvpn.block.key", "openvpn.block.tls-auth"]
[settings]
ipv6_block = true
auto_reconnect = true
ignore_pushed_routes = false
[openvpn]
config = "client\nremote vpn.esempio-ufficio.it 1194\n"
"#;

#[test]
fn to_connect_params_reads_through_the_layers() {
    let w = World::new();
    let p = Profile::from_toml(EXAMPLE).unwrap();
    w.keyring
        .set(
            &p.id,
            "openvpn.block.key",
            &Secret::new("private-test-key\n"),
        )
        .unwrap();
    w.file()
        .set(
            &p.id,
            "openvpn.block.tls-auth",
            &Secret::new("static-test-key\n"),
        )
        .unwrap();
    for writes in [Writes::Keyring, Writes::File] {
        let params = to_connect_params(&p, &w.layered(writes), false).unwrap();
        let galleria_ipc::BackendParams::Openvpn(o) = params.backend else {
            panic!("not an OpenVPN profile")
        };
        assert_eq!(o.secret_blocks["key"].expose(), "private-test-key\n");
        assert_eq!(o.secret_blocks["tls-auth"].expose(), "static-test-key\n");
    }
}

#[test]
fn file_store_delete_keeps_the_file_private() {
    let dir = private_dir();
    let store = FileSecretStore::open(dir.path()).unwrap();
    let id = Uuid::new_v4();
    store.set(&id, "a", &Secret::new("one")).unwrap();
    store.set(&id, "b", &Secret::new("two")).unwrap();
    store.delete(&id, "a").unwrap();
    let path = dir.path().join(format!("secrets/{id}.toml"));
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    assert_eq!(read(&store, &id, "a"), None);
    assert_eq!(read(&store, &id, "b").as_deref(), Some("two"));
    // the last key out takes the file with it, so no empty file of secrets is left behind
    store.delete(&id, "b").unwrap();
    assert!(!path.exists());
    store.delete(&id, "b").unwrap();
}

#[test]
fn a_keyring_error_never_carries_a_value() {
    let seed = "SEED-SECRET-0123456789";
    let errors = [
        StoreError::KeyringLocked,
        StoreError::Keyring("org.freedesktop.DBus.Error.NoReply".into()),
    ];
    for e in &errors {
        for text in [e.to_string(), format!("{e:?}")] {
            assert!(!text.contains("SEED-SECRET-"), "{text}");
        }
    }
    // and the layers do not add the value to the errors they pass on
    let w = World::new();
    let id = Uuid::new_v4();
    w.keyring.lock(true);
    let err = w
        .layered(Writes::Keyring)
        .set(&id, "k", &Secret::new(seed))
        .unwrap_err();
    for text in [err.to_string(), format!("{err:?}")] {
        assert!(!text.contains("SEED-SECRET-"), "{text}");
    }
}
