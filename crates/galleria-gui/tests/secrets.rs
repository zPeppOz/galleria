//! The archive of the secrets in the window (plan B5, T3): what an import, a copy and a delete do to the keyring and to the
//! file, that a locked keyring stops them without leaving half a profile, that nothing here asks for an unlock before a
//! gesture, and that no command returns a secret when the secrets are in the keyring. The keyring is an archive in
//! memory that counts what is asked of it; the real one is tried by `scripts/gui.sh keyring` and by the smoke test.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/seed.rs"]
mod seed;

use fixtures::{Fixtures, write_fixtures};
use galleria_gui::{
    configure_with,
    error::CmdError,
    export::SavePicker,
    import::{ImportFailure, Imports, Origin},
    rubrica::{Rubrica, RubricaError, SecretFactory, StoreProbe},
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use galleria_ipc::Secret;
use galleria_profile::{
    FileSecretStore, LayeredSecretStore, ProfileStore, SecretStore, StoreError, StoreKind, Writes,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};
use uuid::Uuid;

#[derive(Default)]
struct Memory {
    values: BTreeMap<(Uuid, String), String>,
    locked: bool,
    /// The n-th write (counting from 1) is refused with `KeyringLocked`, once, and changes nothing.
    refuse_write: Option<usize>,
    reads: usize,
    writes: usize,
    /// How many times a request to unlock would have been made: every call that finds the keyring locked.
    unlock_requests: usize,
}

/// A keyring in memory that counts what is asked of it.
#[derive(Clone, Default)]
struct FakeKeyring(Arc<Mutex<Memory>>);

impl FakeKeyring {
    fn locked() -> Self {
        let k = Self::default();
        k.0.lock().unwrap().locked = true;
        k
    }
    fn refusing_write(n: usize) -> Self {
        let k = Self::default();
        k.0.lock().unwrap().refuse_write = Some(n);
        k
    }
    fn lock(&self, locked: bool) {
        self.0.lock().unwrap().locked = locked;
    }
    fn count(&self) -> (usize, usize, usize) {
        let m = self.0.lock().unwrap();
        (m.reads, m.writes, m.unlock_requests)
    }
    fn keys_of(&self, id: &Uuid) -> Vec<String> {
        let m = self.0.lock().unwrap();
        m.values
            .keys()
            .filter(|(i, _)| i == id)
            .map(|(_, k)| k.clone())
            .collect()
    }
    fn len(&self) -> usize {
        self.0.lock().unwrap().values.len()
    }
    fn everything(&self) -> String {
        format!("{:?}", self.0.lock().unwrap().values)
    }
}

impl SecretStore for FakeKeyring {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError> {
        let mut m = self.0.lock().unwrap();
        if m.locked {
            m.unlock_requests += 1;
            return Err(StoreError::KeyringLocked);
        }
        m.reads += 1;
        Ok(m.values.get(&(*id, key.into())).map(Secret::new))
    }
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError> {
        let mut m = self.0.lock().unwrap();
        if m.locked {
            m.unlock_requests += 1;
            return Err(StoreError::KeyringLocked);
        }
        m.writes += 1;
        if m.refuse_write == Some(m.writes) {
            m.refuse_write = None;
            return Err(StoreError::KeyringLocked);
        }
        m.values.insert((*id, key.into()), value.expose().into());
        Ok(())
    }
    fn delete(&self, id: &Uuid, key: &str) -> Result<(), StoreError> {
        let mut m = self.0.lock().unwrap();
        if m.locked {
            m.unlock_requests += 1;
            return Err(StoreError::KeyringLocked);
        }
        m.values.remove(&(*id, key.into()));
        Ok(())
    }
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError> {
        let mut m = self.0.lock().unwrap();
        if m.locked {
            m.unlock_requests += 1;
            return Err(StoreError::KeyringLocked);
        }
        m.values.retain(|(i, _), _| i != id);
        Ok(())
    }
}

fn factory(keyring: Option<FakeKeyring>) -> SecretFactory {
    Arc::new(move |dir: &Path, _name: &str| {
        let keyring = keyring.clone().map(|k| Box::new(k) as Box<dyn SecretStore>);
        Ok(Box::new(LayeredSecretStore::new(
            keyring,
            Box::new(FileSecretStore::open(dir)?),
            Writes::Keyring,
        )) as Box<dyn SecretStore>)
    })
}

fn probe(keyring: Option<FakeKeyring>) -> StoreProbe {
    Arc::new(move || {
        if keyring.is_some() {
            StoreKind::Keyring
        } else {
            StoreKind::File
        }
    })
}

struct World {
    root: tempfile::TempDir,
    rubrica: Arc<Rubrica>,
    files: Fixtures,
    keyring: Option<FakeKeyring>,
}

impl World {
    fn new(keyring: Option<FakeKeyring>) -> Self {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(UiStateFile::new(
            root.path().join("state/galleria/state.toml"),
        ));
        let rubrica = Arc::new(Rubrica::with_archive(
            root.path().join("galleria"),
            state,
            factory(keyring.clone()),
            probe(keyring.clone()),
        ));
        let files = write_fixtures(&root.path().join("files"));
        Self {
            root,
            rubrica,
            files,
            keyring,
        }
    }
    fn seeded(keyring: Option<FakeKeyring>) -> (Self, seed::Seeded) {
        let world = Self::new(keyring);
        let seeded = seed::seed(&world.dir(), false);
        (world, seeded)
    }
    fn dir(&self) -> PathBuf {
        self.root.path().join("galleria")
    }
    fn keyring(&self) -> &FakeKeyring {
        self.keyring.as_ref().unwrap()
    }
    fn secret_files(&self) -> Vec<String> {
        names_in(&self.dir().join("secrets"))
    }
    fn profile_files(&self) -> Vec<String> {
        names_in(&self.dir().join("profiles"))
    }
    fn file_store(&self) -> FileSecretStore {
        FileSecretStore::open(&self.dir()).unwrap()
    }
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn id_of(seeded: &seed::Seeded, name: &str) -> Uuid {
    seeded.profiles.iter().find(|(n, _)| n == name).unwrap().1
}

/// The ids of the profiles that are saved.
fn saved(world: &World) -> Vec<Uuid> {
    world
        .rubrica
        .list()
        .unwrap()
        .profiles
        .iter()
        .map(|p| p.id)
        .collect()
}

struct Saves(Option<PathBuf>);
impl SavePicker for Saves {
    fn pick(&self, _suggested: &str) -> Option<PathBuf> {
        self.0.clone()
    }
}

fn import_inline(world: &World, name: &str) -> Result<Uuid, ImportFailure> {
    let imports = Imports::new(Arc::clone(&world.rubrica));
    let preview = imports
        .offer_path(&world.files.inline, Origin::Picker)
        .unwrap();
    imports.commit(preview.id, name, None).map(|p| p.id)
}

#[test]
fn import_commit_writes_the_keyring_and_no_secret_file() {
    let world = World::new(Some(FakeKeyring::default()));
    let id = import_inline(&world, "Keyring").unwrap();
    // the key and the tls-crypt block are in the keyring, under the id of the profile
    let mut keys = world.keyring().keys_of(&id);
    keys.sort();
    assert_eq!(keys, ["openvpn.block.key", "openvpn.block.tls-crypt"]);
    assert!(
        !world
            .dir()
            .join("secrets")
            .join(format!("{id}.toml"))
            .exists(),
        "no file of secrets for the profile"
    );
    assert!(
        world.secret_files().is_empty(),
        "{:?}",
        world.secret_files()
    );
    // and the profile reads them through the layers
    assert_eq!(world.profile_files(), [format!("{id}.toml")]);
}

#[test]
fn without_a_keyring_import_writes_the_file_and_the_status_says_file() {
    let world = World::new(None);
    assert_eq!(world.rubrica.secret_store_kind(), StoreKind::File);
    let id = import_inline(&world, "File").unwrap();
    assert_eq!(world.secret_files(), [format!("{id}.toml")]);
    let (with, _) = World::seeded(Some(FakeKeyring::default()));
    assert_eq!(with.rubrica.secret_store_kind(), StoreKind::Keyring);
}

#[test]
fn a_locked_keyring_stops_the_import_and_leaves_nothing() {
    let world = World::new(Some(FakeKeyring::locked()));
    let before = (world.profile_files(), world.secret_files());
    let error = import_inline(&world, "Locked").unwrap_err();
    assert!(
        matches!(
            error,
            ImportFailure::Rubrica(RubricaError::Store {
                source: StoreError::KeyringLocked,
                ..
            })
        ),
        "{error:?}"
    );
    let command = CmdError::from(error);
    assert_eq!(command.code, "gui_keyring_locked");
    assert!(!command.message.contains("SEED-SECRET-"));
    assert_eq!(
        (world.profile_files(), world.secret_files()),
        before,
        "no profile and no secret file"
    );
    assert_eq!(world.keyring().len(), 0, "nothing in the keyring");
    // not the file either: a keyring that is there but locked is not a reason to write the file
    assert!(!world.dir().join("secrets").exists() || world.secret_files().is_empty());
}

#[test]
fn another_error_of_the_keyring_has_its_own_code() {
    assert_eq!(
        CmdError::from(RubricaError::Store {
            dir: PathBuf::from("/x"),
            source: StoreError::Keyring("D-Bus error org.freedesktop.DBus.Error.NoReply".into()),
        })
        .code,
        "gui_keyring_failed"
    );
}

#[test]
fn a_duplicate_stopped_halfway_leaves_nothing_in_either_store() {
    // the keyring takes the first secret of the copy and refuses the second
    let keyring = FakeKeyring::refusing_write(2);
    let (world, seeded) = World::seeded(Some(keyring));
    let original = id_of(&seeded, "Ufficio Napoli");
    let profiles = world.profile_files();
    let secrets = world.secret_files();
    let error = world.rubrica.duplicate(original, "Copia").unwrap_err();
    assert!(
        matches!(
            error,
            RubricaError::Store {
                source: StoreError::KeyringLocked,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(world.profile_files(), profiles, "no profile was left");
    assert_eq!(world.secret_files(), secrets, "no file of secrets was left");
    assert_eq!(world.keyring().len(), 0, "what was written first was wiped");
    // the original is as it was, and the copy works once the keyring answers
    assert!(
        world
            .file_store()
            .get(&original, "openvpn.block.tls-crypt")
            .unwrap()
            .is_some()
    );
    let copy = world.rubrica.duplicate(original, "Copia").unwrap();
    assert_eq!(world.keyring().keys_of(&copy.id).len(), 2);
    assert!(
        !world
            .dir()
            .join("secrets")
            .join(format!("{}.toml", copy.id))
            .exists()
    );
}

#[test]
fn delete_removes_the_keyring_items_too() {
    let (world, seeded) = World::seeded(Some(FakeKeyring::default()));
    let id = import_inline(&world, "Both").unwrap();
    // a profile that has keys in the two archives, as one that was imported by the CLI and then saved again
    let seeded_id = id_of(&seeded, "Ufficio Napoli");
    world
        .keyring()
        .set(&seeded_id, "openvpn.block.key", &Secret::new("in-keyring"))
        .unwrap();
    assert_eq!(world.keyring().keys_of(&id).len(), 2);
    world.rubrica.delete(id, None).unwrap();
    world.rubrica.delete(seeded_id, None).unwrap();
    assert_eq!(world.keyring().len(), 0, "{}", world.keyring().everything());
    assert!(!world.secret_files().contains(&format!("{seeded_id}.toml")));
    assert!(!saved(&world).contains(&id) && !saved(&world).contains(&seeded_id));
}

#[test]
fn a_locked_keyring_stops_the_delete_and_leaves_the_profile() {
    let (world, seeded) = World::seeded(Some(FakeKeyring::default()));
    let id = id_of(&seeded, "Homelab");
    world.keyring().lock(true);
    let error = world.rubrica.delete(id, None).unwrap_err();
    assert_eq!(CmdError::from(error).code, "gui_keyring_locked");
    assert!(saved(&world).contains(&id), "the profile is still there");
    world.keyring().lock(false);
    world.rubrica.delete(id, None).unwrap();
}

#[test]
fn a_profile_imported_by_the_cli_is_read_from_the_file() {
    // the seed writes the secrets the way the CLI does: in the file; the keyring has nothing
    let (world, seeded) = World::seeded(Some(FakeKeyring::default()));
    let id = id_of(&seeded, "Ufficio Napoli");
    assert_eq!(world.keyring().len(), 0);
    let out = world.root.path().join("exported.ovpn");
    assert!(world.rubrica.export(id, &Saves(Some(out.clone()))).unwrap());
    let text = fs::read_to_string(&out).unwrap();
    assert!(
        text.contains("SEED-SECRET-"),
        "the key of the file is in the export"
    );
    let copy = world.rubrica.duplicate(id, "Copia").unwrap();
    // the copy is written the window's way: in the keyring, and not in the file
    assert_eq!(world.keyring().keys_of(&copy.id).len(), 2);
    assert!(!world.secret_files().contains(&format!("{}.toml", copy.id)));
    // and a key that is in both places is the keyring's: the export has it
    world
        .keyring()
        .set(
            &id,
            "openvpn.block.tls-crypt",
            &Secret::new("SEED-SECRET-from-keyring"),
        )
        .unwrap();
    fs::remove_file(&out).unwrap();
    world.rubrica.export(id, &Saves(Some(out.clone()))).unwrap();
    assert!(
        fs::read_to_string(&out)
            .unwrap()
            .contains("SEED-SECRET-from-keyring")
    );
}

#[test]
fn secret_store_status_never_reads_a_secret() {
    for keyring in [FakeKeyring::default(), FakeKeyring::locked()] {
        let (world, seeded) = World::seeded(Some(keyring));
        let id = id_of(&seeded, "Ufficio Napoli");
        assert_eq!(world.rubrica.secret_store_kind(), StoreKind::Keyring);
        world.rubrica.list().unwrap();
        world.rubrica.get(id).unwrap();
        assert_eq!(world.keyring().count(), (0, 0, 0));
    }
}

struct Gui {
    _app: tauri::App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
}

fn gui(world: &World) -> Gui {
    let mut app = configure_with(
        mock_builder().manage(Arc::clone(&world.rubrica)),
        world.root.path().join("no-daemon.sock"),
        RelinkPolicy {
            delays: vec![Duration::from_millis(20)],
        },
        UiStateFile::new(world.root.path().join("state/galleria/state.toml")),
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    Gui { _app: app, window }
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
fn starting_the_app_never_asks_to_unlock() {
    // a keyring that is locked: starting the app, the list, a profile and the status make no request to unlock and read nothing
    let (world, seeded) = World::seeded(Some(FakeKeyring::locked()));
    let gui = gui(&world);
    let id = id_of(&seeded, "Ufficio Napoli");
    let list = invoke(&gui.window, "profiles_list", json!({})).unwrap();
    assert_eq!(list["profiles"].as_array().unwrap().len(), 5);
    invoke(&gui.window, "profile_get", json!({ "id": id })).unwrap();
    let status = invoke(&gui.window, "secret_store_status", json!({})).unwrap();
    assert_eq!(status, json!({ "store": "keyring" }));
    assert_eq!(
        world.keyring().count(),
        (0, 0, 0),
        "nothing was asked of the keyring"
    );
}

#[test]
fn secret_store_status_says_where_a_write_goes() {
    let world = World::new(None);
    let gui = gui(&world);
    assert_eq!(
        invoke(&gui.window, "secret_store_status", json!({})).unwrap(),
        json!({ "store": "file" })
    );
    let with = World::new(Some(FakeKeyring::default()));
    let gui = self::gui(&with);
    assert_eq!(
        invoke(&gui.window, "secret_store_status", json!({})).unwrap(),
        json!({ "store": "keyring" })
    );
}

#[test]
fn no_command_returns_a_secret_with_the_secrets_in_the_keyring() {
    let (world, seeded) = World::seeded(Some(FakeKeyring::default()));
    let gui = gui(&world);
    let mut seen: Vec<(String, String)> = vec![];
    let mut note = |what: &str, answer: Result<Value, Value>| {
        let value = answer.unwrap_or_else(|e| e);
        seen.push((what.to_owned(), value.to_string()));
        value
    };
    // the secrets of the profiles are in the keyring, all of them marked
    for (name, id) in &seeded.profiles {
        for key in world.file_store_keys(id) {
            let value = world.file_store().get(id, &key).unwrap().unwrap();
            world.keyring().set(id, &key, &value).unwrap();
        }
        let _ = name;
    }
    assert!(world.keyring().everything().contains("SEED-SECRET-"));
    let id = id_of(&seeded, "Ufficio Napoli");
    note(
        "profiles_list",
        invoke(&gui.window, "profiles_list", json!({})),
    );
    note(
        "profile_get",
        invoke(&gui.window, "profile_get", json!({ "id": id })),
    );
    note(
        "secret_store_status",
        invoke(&gui.window, "secret_store_status", json!({})),
    );
    let text = fs::read_to_string(&world.files.inline).unwrap();
    let preview = note(
        "import_from_text",
        invoke(&gui.window, "import_from_text", json!({ "text": text })),
    );
    note(
        "import_commit",
        invoke(
            &gui.window,
            "import_commit",
            json!({ "id": preview["id"], "name": "In keyring", "group": null }),
        ),
    );
    note(
        "profile_duplicate",
        invoke(
            &gui.window,
            "profile_duplicate",
            json!({ "id": id, "name": "Copy" }),
        ),
    );
    // a locked keyring: the errors carry no secret either
    world.keyring().lock(true);
    note(
        "profile_duplicate (locked)",
        invoke(
            &gui.window,
            "profile_duplicate",
            json!({ "id": id, "name": "Copy two" }),
        ),
    );
    note(
        "profile_delete (locked)",
        invoke(&gui.window, "profile_delete", json!({ "id": id })),
    );
    for (what, answer) in &seen {
        for needle in ["SEED-SECRET-", "PRIVATE KEY", "in-keyring"] {
            assert!(
                !answer.contains(needle),
                "{what} returned {needle}: {answer}"
            );
        }
    }
    let locked = seen
        .iter()
        .find(|(w, _)| w == "profile_duplicate (locked)")
        .unwrap();
    assert!(locked.1.contains("gui_keyring_locked"), "{}", locked.1);
}

impl World {
    /// The keys the file holds for a profile: its `secret_refs`.
    fn file_store_keys(&self, id: &Uuid) -> Vec<String> {
        ProfileStore::open(&self.dir())
            .unwrap()
            .find(&id.to_string())
            .unwrap()
            .secret_refs
    }
}
