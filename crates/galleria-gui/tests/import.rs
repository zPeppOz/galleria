//! Import in the window (plan B4, T1): the previews that wait in memory, what is written only at «Import», and that nothing
//! that is a secret, a path or the text of a profile crosses the commands. The files are the fixtures of
//! `support/fixtures.rs`; the window «Open» and the drop are tried in the later tasks.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/seed.rs"]
mod seed;

use fixtures::{Fixtures, SECRET_MARK, write_fixtures};
use galleria_gui::{
    configure_with,
    error::CmdError,
    import::{
        ImportFailure, ImportOffer, ImportPreview, Imports, MAX_PREVIEWS, NoticeKind, Origin,
        PREVIEW_TTL,
    },
    picker::FilePicker,
    rubrica::{Rubrica, RubricaError},
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use galleria_ipc::{Protocol, Secret};
use galleria_profile::{FileSecretStore, ProfileStore, SecretStore, StoreError, import_file};
use rstest::rstest;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    Listener, Manager, WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};
use uuid::Uuid;

struct World {
    root: tempfile::TempDir,
    rubrica: Arc<Rubrica>,
    files: Fixtures,
}

impl World {
    /// An empty folder of the profiles (it does not exist yet) and the fixtures in a folder of their own.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(UiStateFile::new(
            root.path().join("state/galleria/state.toml"),
        ));
        let rubrica = Arc::new(Rubrica::new(root.path().join("galleria"), state));
        let files = write_fixtures(&root.path().join("files"));
        Self {
            root,
            rubrica,
            files,
        }
    }

    fn seeded() -> Self {
        let world = Self::new();
        seed::seed(&world.dir(), false);
        world
    }

    fn dir(&self) -> PathBuf {
        self.root.path().join("galleria")
    }

    fn imports(&self) -> Imports {
        Imports::new(Arc::clone(&self.rubrica))
    }

    /// The imports with a clock that the test moves.
    fn imports_with_clock(&self) -> (Imports, Arc<Mutex<Instant>>) {
        let now = Arc::new(Mutex::new(Instant::now()));
        let clock = Arc::clone(&now);
        let imports = Imports::with_clock(
            Arc::clone(&self.rubrica),
            Arc::new(move || *clock.lock().unwrap()),
        );
        (imports, now)
    }
}

/// Every file and folder under `dir` with its bytes: what a read must leave as it was.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(base).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(relative, None);
                walk(base, &path, out);
            } else {
                out.insert(relative, Some(fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn files_in(dir: &Path) -> Vec<String> {
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

/// What a command answers must hold no secret, no key and no folder: the text of the shown value, searched.
fn assert_clean(value: &impl Serialize, folders: &[&Path]) {
    let shown = serde_json::to_string(value).unwrap();
    assert!(!shown.contains(SECRET_MARK), "a secret is in {shown}");
    assert!(!shown.contains("PRIVATE KEY"), "a key is in {shown}");
    assert!(!shown.contains("BEGIN"), "a block is in {shown}");
    for folder in folders {
        assert!(
            !shown.contains(&*folder.to_string_lossy()),
            "{folder:?} is in {shown}"
        );
    }
}

fn error_of(failure: ImportFailure) -> Value {
    serde_json::to_value(CmdError::from(failure)).unwrap()
}

fn offer_error(result: Result<ImportPreview, ImportFailure>) -> ImportFailure {
    match result {
        Ok(preview) => panic!("a preview was made: {preview:?}"),
        Err(failure) => failure,
    }
}

// ---- the preview ----------------------------------------------------------------------------------------------------

#[test]
fn preview_names_the_secrets_and_never_their_values() {
    let world = World::new();
    let imports = world.imports();
    let preview = imports
        .offer_path(&world.files.inline, Origin::Picker)
        .unwrap();
    assert_eq!(preview.protocol, Protocol::Openvpn);
    assert_eq!(preview.origin, Origin::Picker);
    assert_eq!(preview.file_name.as_deref(), Some("inline.ovpn"));
    assert_eq!(preview.suggested_name, "inline");
    assert_eq!(preview.server.as_deref(), Some("vpn.fixture.example:1194"));
    assert!(preview.auth.user_pass && preview.auth.certificate && !preview.auth.otp);
    let mut found = preview.found.clone();
    found.sort();
    assert_eq!(found, ["openvpn.block.key", "openvpn.block.tls-crypt"]);
    assert_eq!(preview.removed, 0);
    assert_eq!(preview.expires_in_secs, PREVIEW_TTL.as_secs());
    assert_clean(&preview, &[world.root.path()]);

    // pasted text: no file name, and the same names
    let text = fs::read_to_string(&world.files.inline).unwrap();
    let pasted = imports.offer_text(&text).unwrap();
    assert_eq!(pasted.origin, Origin::Text);
    assert_eq!(pasted.file_name, None);
    assert_eq!(pasted.found.len(), 2);
    assert_clean(&pasted, &[world.root.path()]);

    // WireGuard: the keys are named, not shown
    let office = imports
        .offer_path(&world.files.office, Origin::Picker)
        .unwrap();
    assert_eq!(office.protocol, Protocol::Wireguard);
    let mut found = office.found.clone();
    found.sort();
    assert_eq!(
        found,
        ["wireguard.peer.0.preshared_key", "wireguard.private_key"]
    );
    assert_eq!(
        office.server.as_deref(),
        Some("office.fixture.example:51820")
    );
    assert_clean(&office, &[world.root.path()]);
}

#[test]
fn a_preview_names_the_file_and_never_its_folder() {
    let world = World::new();
    let preview = world
        .imports()
        .offer_path(&world.files.separate, Origin::Drop)
        .unwrap();
    assert_eq!(preview.file_name.as_deref(), Some("separate.ovpn"));
    assert_eq!(preview.origin, Origin::Drop);
    assert_clean(
        &preview,
        &[world.root.path(), world.files.separate.parent().unwrap()],
    );
}

#[test]
fn no_import_command_returns_a_secret() {
    let world = World::seeded();
    let imports = world.imports();
    let folders = [world.root.path()];
    // every file of the fixtures, as a file and as text
    for path in [
        &world.files.inline,
        &world.files.separate,
        &world.files.malicious,
        &world.files.dns,
        &world.files.office,
        &world.files.server,
    ] {
        let preview = imports.offer_path(path, Origin::Open).unwrap();
        assert_clean(&preview, &folders);
        let saved = imports.commit(preview.id, "Saved", None).unwrap();
        assert_clean(&saved, &folders);
    }
    let text = fs::read_to_string(&world.files.malicious).unwrap();
    assert_clean(&imports.offer_text(&text).unwrap(), &folders);
    // every kind of error, with a text that has a secret in it
    let broken = [
        format!("client\n<key>\n{SECRET_MARK}broken\n"),
        format!("client\nca {SECRET_MARK}file.crt\n"),
        format!("client\nca ../{SECRET_MARK}file.crt\n"),
        format!("{SECRET_MARK}\n[Interface]\n"),
        "x".repeat(2 * 1024 * 1024),
    ];
    for text in &broken {
        if let Err(failure) = imports.offer_text(text) {
            let error = error_of(failure);
            // the file a profile refers to is named in `data.file`; it is the one thing of a text that comes back
            let mut shown = error.to_string();
            if let Some(file) = error["data"]["file"].as_str() {
                shown = shown.replace(file, "");
            }
            assert!(!shown.contains("BEGIN"), "{shown}");
            assert!(!shown.contains(SECRET_MARK), "{shown}");
            assert!(!error["message"].as_str().unwrap().contains(SECRET_MARK));
        }
    }
    for path in [
        &world.files.outside,
        &world.root.path().join("nothing.ovpn"),
    ] {
        let error = error_of(offer_error(imports.offer_path(path, Origin::Open)));
        assert_clean(&error, &folders);
    }
    let error = error_of(imports.commit(Uuid::new_v4(), "x", None).unwrap_err());
    assert_clean(&error, &folders);
}

#[test]
fn an_error_never_echoes_the_text() {
    let world = World::new();
    let imports = world.imports();
    // a block that never closes, with a secret in it: the error has the line, and nothing of the text
    let text = format!("client\nremote x 1194\n<key>\n{SECRET_MARK}echo\n");
    let failure = offer_error(imports.offer_text(&text));
    let error = error_of(failure);
    assert_eq!(error["code"], "import_syntax");
    assert!(error["data"]["line"].as_u64().is_some());
    assert!(!error.to_string().contains(SECRET_MARK), "{error}");
    // the same for a WireGuard text
    let text = format!("[Interface]\nPrivateKey {SECRET_MARK}no-equal\n");
    let error = error_of(offer_error(imports.offer_text(&text)));
    assert_eq!(error["code"], "import_syntax");
    assert_eq!(error["data"], json!({ "line": 2 }));
    assert!(!error.to_string().contains(SECRET_MARK), "{error}");
    // and for a WireGuard text that is incomplete
    let text = format!("[Interface]\nPrivateKey = {SECRET_MARK}alone\n");
    let error = error_of(offer_error(imports.offer_text(&text)));
    assert_eq!(error["code"], "import_invalid");
    assert!(!error.to_string().contains(SECRET_MARK), "{error}");
}

#[test]
fn the_errors_have_the_codes_of_the_catalogue() {
    let world = World::new();
    let imports = world.imports();
    let code = |failure| error_of(failure)["code"].as_str().unwrap().to_owned();
    // a reference in pasted text: no folder to read it from
    let failure = offer_error(imports.offer_text("client\nca ca.crt\n"));
    let error = error_of(failure);
    assert_eq!(error["code"], "import_missing_file");
    assert_eq!(error["data"], json!({ "file": "ca.crt", "origin": "text" }));
    assert_eq!(
        code(offer_error(imports.offer_text("client\nca ../ca.crt\n"))),
        "import_outside_folder"
    );
    assert_eq!(
        code(offer_error(
            imports.offer_text(&"#".repeat(1024 * 1024 + 1))
        )),
        "gui_import_too_large"
    );
    // a file
    let outside = offer_error(imports.offer_path(&world.files.outside, Origin::Picker));
    assert_eq!(code(outside), "import_outside_folder");
    let separate_without_files = world.root.path().join("alone/separate.ovpn");
    fs::create_dir_all(separate_without_files.parent().unwrap()).unwrap();
    fs::copy(&world.files.separate, &separate_without_files).unwrap();
    let error = error_of(offer_error(
        imports.offer_path(&separate_without_files, Origin::Drop),
    ));
    assert_eq!(error["code"], "import_missing_file");
    assert_eq!(error["data"]["origin"], "drop");
    let missing = world.root.path().join("none.ovpn");
    let error = error_of(offer_error(imports.offer_path(&missing, Origin::Picker)));
    assert_eq!(error["code"], "gui_import_unreadable");
    assert_eq!(error["data"], json!({ "file_name": "none.ovpn" }));
    let text = world.root.path().join("notes.txt");
    fs::write(&text, "client\n").unwrap();
    let error = error_of(offer_error(imports.offer_path(&text, Origin::Picker)));
    assert_eq!(error["code"], "gui_import_unsupported_file");
    assert_eq!(error["data"], json!({ "file_name": "notes.txt" }));
    // a folder with the right extension, and a huge file
    let folder = world.root.path().join("folder.ovpn");
    fs::create_dir(&folder).unwrap();
    assert_eq!(
        code(offer_error(imports.offer_path(&folder, Origin::Drop))),
        "gui_import_unsupported_file"
    );
    let huge = world.root.path().join("huge.ovpn");
    fs::write(&huge, vec![b'#'; 2 * 1024 * 1024]).unwrap();
    assert_eq!(
        code(offer_error(imports.offer_path(&huge, Origin::Drop))),
        "gui_import_too_large"
    );
}

// ---- what the import removes ----------------------------------------------------------------------------------------

/// The line (from 1) of the first line of the file that starts with `directive `.
fn line_of(path: &Path, directive: &str) -> usize {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .position(|l| l.split_whitespace().next() == Some(directive))
        .unwrap()
        + 1
}

#[test]
fn the_malicious_profile_lists_every_removed_directive() {
    let world = World::new();
    let preview = world
        .imports()
        .offer_path(&world.files.malicious, Origin::Picker)
        .unwrap();
    // What the import of the phase 1 reports: the dangerous and the unsupported ones. `dev`, `verb`, `script-security` and
    // `management` belong to the daemon and the import drops them without a word (spec §8.2): they are not in the list.
    let mut names: Vec<&str> = preview
        .notices
        .iter()
        .map(|n| n.directive.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["plugin", "setenv", "status", "up", "writepid"]);
    for notice in &preview.notices {
        assert_eq!(notice.kind, NoticeKind::Dangerous, "{}", notice.directive);
        assert_eq!(
            notice.line,
            line_of(&world.files.malicious, &notice.directive),
            "{}",
            notice.directive
        );
        assert!(!notice.dns_script, "{}", notice.directive);
    }
    assert_eq!(preview.removed, 5);
}

#[test]
fn a_notice_never_carries_an_argument() {
    let world = World::new();
    let imports = world.imports();
    let preview = imports
        .offer_path(&world.files.malicious, Origin::Picker)
        .unwrap();
    let shown = serde_json::to_string(&preview).unwrap();
    for argument in ["/tmp/PWNED", "7505", "EVIL", "/usr/bin/touch", "0.0.0.0"] {
        assert!(!shown.contains(argument), "{argument} is in {shown}");
    }
    // the same for a WireGuard hook
    let office = imports
        .offer_path(&world.files.office, Origin::Picker)
        .unwrap();
    assert!(!serde_json::to_string(&office).unwrap().contains("PWNED"));
}

#[rstest]
#[case::with_its_folder("up /etc/openvpn/update-resolv-conf\n", true)]
#[case::the_other_one("up /etc/openvpn/update-systemd-resolved\n", true)]
#[case::without_a_folder("up update-resolv-conf\n", true)]
#[case::with_an_extension("up /etc/openvpn/update-resolv-conf.sh\n", true)]
#[case::with_arguments("up \"/etc/openvpn/update-systemd-resolved\" -v\n", true)]
#[case::in_down_too("down /etc/openvpn/update-resolv-conf\n", true)]
#[case::another_command("up /usr/bin/touch /tmp/PWNED-up\n", false)]
#[case::a_name_that_only_starts_alike("up /tmp/update-resolv-conf-evil\n", false)]
#[case::a_plugin("plugin /usr/lib/openvpn/openvpn-plugin-down-root.so\n", false)]
#[case::a_directive_that_is_not_a_script("status /tmp/update-resolv-conf\n", false)]
fn dns_script_is_true_only_for_the_two_resolver_scripts(#[case] line: &str, #[case] dns: bool) {
    let world = World::new();
    let text = format!("client\nremote vpn.example 1194\n{line}");
    let preview = world.imports().offer_text(&text).unwrap();
    let notice = preview
        .notices
        .iter()
        .find(|n| n.kind == NoticeKind::Dangerous)
        .unwrap_or_else(|| panic!("nothing was removed: {:?}", preview.notices));
    // `status` is no script: only a directive that runs one may say yes
    let runs_a_script = matches!(notice.directive.as_str(), "up" | "down");
    assert_eq!(
        notice.dns_script,
        dns && runs_a_script,
        "{}",
        notice.directive
    );
}

#[test]
fn the_resolver_scripts_of_a_profile_are_told_apart_in_the_file_too() {
    let world = World::new();
    let preview = world
        .imports()
        .offer_path(&world.files.dns, Origin::Open)
        .unwrap();
    let scripts: Vec<(&str, bool)> = preview
        .notices
        .iter()
        .map(|n| (n.directive.as_str(), n.dns_script))
        .collect();
    assert_eq!(scripts, [("up", true), ("down", true)]);
    assert_eq!(preview.removed, 2);
}

#[test]
fn wireguard_hooks_are_dangerous_notices() {
    let world = World::new();
    let preview = world
        .imports()
        .offer_path(&world.files.office, Origin::Picker)
        .unwrap();
    assert_eq!(preview.notices.len(), 1);
    let notice = &preview.notices[0];
    assert_eq!(notice.directive, "PostUp");
    assert_eq!(notice.kind, NoticeKind::Dangerous);
    assert_eq!(
        notice.line,
        line_of_wireguard(&world.files.office, "PostUp")
    );
    assert!(!notice.dns_script);
    assert_eq!(preview.removed, 1);
}

fn line_of_wireguard(path: &Path, key: &str) -> usize {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .position(|l| l.starts_with(key))
        .unwrap()
        + 1
}

#[test]
fn a_missing_client_is_noticed_and_not_counted() {
    let world = World::new();
    let preview = world
        .imports()
        .offer_text("remote vpn.example 1194\n")
        .unwrap();
    assert_eq!(preview.notices.len(), 1);
    assert_eq!(preview.notices[0].kind, NoticeKind::NoTlsClient);
    assert_eq!(preview.removed, 0);
}

// ---- nothing is written before «Import» -------------------------------------------------------------------------------

#[test]
fn nothing_is_written_before_commit() {
    for seeded in [false, true] {
        let world = if seeded {
            World::seeded()
        } else {
            World::new()
        };
        let imports = world.imports();
        let before = snapshot(world.root.path());
        let mut ids = vec![];
        for path in [
            &world.files.inline,
            &world.files.separate,
            &world.files.malicious,
            &world.files.office,
        ] {
            ids.push(imports.offer_path(path, Origin::Picker).unwrap().id);
        }
        let text = fs::read_to_string(&world.files.inline).unwrap();
        ids.push(imports.offer_text(&text).unwrap().id);
        // refused offers leave nothing either
        let _ = imports.offer_path(&world.files.outside, Origin::Drop);
        let _ = imports.offer_text("client\nca ca.crt\n");
        assert_eq!(snapshot(world.root.path()), before, "seeded: {seeded}");
        for id in ids {
            imports.discard(id);
        }
        assert_eq!(snapshot(world.root.path()), before, "seeded: {seeded}");
        if !seeded {
            assert!(!world.dir().exists(), "the folder of the profiles was made");
        }
    }
}

#[test]
fn commit_writes_what_the_cli_import_writes() {
    let world = World::seeded();
    let imports = world.imports();
    for (path, group) in [
        (&world.files.inline, Some("Lavoro")),
        (&world.files.separate, None),
        (&world.files.office, Some("Personale")),
        (&world.files.malicious, None),
    ] {
        let preview = imports.offer_path(path, Origin::Picker).unwrap();
        let summary = imports.commit(preview.id, "Imported", group).unwrap();
        assert_eq!(summary.name, "Imported");
        assert_eq!(summary.group.as_deref(), group);
        assert_eq!(summary.protocol, preview.protocol);
        assert_eq!(summary.server, preview.server);

        // what the CLI writes for the same file
        let cli = import_file(path, Some("Imported")).unwrap();
        let store = ProfileStore::open(&world.dir()).unwrap();
        let saved = store.find(&summary.id.to_string()).unwrap();
        assert_eq!(saved.name, cli.profile.name);
        assert_eq!(saved.protocol, cli.profile.protocol);
        assert_eq!(saved.secret_refs, cli.profile.secret_refs);
        assert_eq!(saved.settings.ipv6_block, cli.profile.settings.ipv6_block);
        assert_eq!(saved.tags, cli.profile.tags);
        assert_eq!(saved.favorite, cli.profile.favorite);
        assert_eq!(saved.notes, cli.profile.notes);
        assert_eq!(
            saved.openvpn.as_ref().map(|o| &o.config),
            cli.profile.openvpn.as_ref().map(|o| &o.config)
        );
        assert_eq!(
            saved
                .wireguard
                .as_ref()
                .map(|w| (w.addresses.clone(), w.peers.len())),
            cli.profile
                .wireguard
                .as_ref()
                .map(|w| (w.addresses.clone(), w.peers.len()))
        );
        // the secrets are in the archive, the same ones
        let secrets = FileSecretStore::open(&world.dir()).unwrap();
        assert!(!cli.secrets.is_empty());
        for (key, value) in &cli.secrets {
            let got = secrets.get(&saved.id, key).unwrap().unwrap();
            assert_eq!(got.expose(), value.expose(), "{key}");
        }
    }
    // the rubrica lists them
    assert_eq!(world.rubrica.list().unwrap().profiles.len(), 5 + 4);
}

/// An archive of the secrets that fails on the n-th write and remembers which ids were wiped; after the last write it can
/// also make the save of the profile fail, by putting a folder where the file of the profile goes.
struct Failing {
    inner: FileSecretStore,
    writes_left: Mutex<usize>,
    wiped: Arc<Mutex<Vec<Uuid>>>,
    block_profile_in: Option<PathBuf>,
}

impl SecretStore for Failing {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError> {
        self.inner.get(id, key)
    }
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError> {
        let mut left = self.writes_left.lock().unwrap();
        if *left == 0 {
            return Err(StoreError::Io(std::io::ErrorKind::PermissionDenied.into()));
        }
        *left -= 1;
        self.inner.set(id, key, value)?;
        if let Some(profiles) = &self.block_profile_in {
            let _ = fs::create_dir_all(profiles.join(format!("{id}.toml")));
        }
        Ok(())
    }
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError> {
        self.wiped.lock().unwrap().push(*id);
        self.inner.delete_all(id)
    }
}

#[rstest]
#[case::the_first_secret(0, false)]
#[case::the_second_secret(1, false)]
#[case::the_profile_itself(usize::MAX, true)]
fn commit_is_all_or_nothing(#[case] writable: usize, #[case] block_the_profile: bool) {
    let world = World::seeded();
    let wiped = Arc::new(Mutex::new(vec![]));
    let seen = Arc::clone(&wiped);
    let profiles_dir = world.dir().join("profiles");
    let blocked = block_the_profile.then(|| profiles_dir.clone());
    let rubrica = Arc::new(Rubrica::with_secrets(
        world.dir(),
        Arc::new(UiStateFile::new(world.root.path().join("state.toml"))),
        Arc::new(move |dir: &Path| {
            Ok(Box::new(Failing {
                inner: FileSecretStore::open(dir)?,
                writes_left: Mutex::new(writable),
                wiped: Arc::clone(&seen),
                block_profile_in: blocked.clone(),
            }) as Box<dyn SecretStore>)
        }),
    ));
    let imports = Imports::new(rubrica);
    let before = (
        files_in(&profiles_dir),
        files_in(&world.dir().join("secrets")),
    );
    // two secrets: the key and the tls-crypt block
    let preview = imports
        .offer_path(&world.files.inline, Origin::Picker)
        .unwrap();
    let error = imports.commit(preview.id, "Copia", None).unwrap_err();
    assert!(
        matches!(error, ImportFailure::Rubrica(RubricaError::Store { .. })),
        "{error:?}"
    );
    assert_eq!(wiped.lock().unwrap().len(), 1, "what was written was wiped");
    if block_the_profile {
        // the folder that stood where the profile goes is the test's own
        for entry in files_in(&profiles_dir) {
            let path = profiles_dir.join(&entry);
            if path.is_dir() {
                fs::remove_dir(path).unwrap();
            }
        }
    }
    assert_eq!(
        (
            files_in(&profiles_dir),
            files_in(&world.dir().join("secrets"))
        ),
        before,
        "no profile and no secret was left"
    );
}

#[test]
fn commit_checks_the_name_and_the_group() {
    let world = World::seeded();
    let imports = world.imports();
    let preview = imports
        .offer_path(&world.files.inline, Origin::Picker)
        .unwrap();
    let before = snapshot(world.root.path());
    for (name, code) in [
        ("", "required"),
        ("   ", "required"),
        (&"x".repeat(65), "too_long"),
        ("a\nb", "invalid_chars"),
    ] {
        match imports.commit(preview.id, name, None).unwrap_err() {
            ImportFailure::Rubrica(RubricaError::Invalid(fields)) => {
                assert_eq!(
                    fields
                        .iter()
                        .map(|f| (f.field.as_str(), f.code))
                        .collect::<Vec<_>>(),
                    [("name", code)],
                    "{name:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }
    // an import does not make groups
    assert!(matches!(
        imports
            .commit(preview.id, "Good", Some("Nowhere"))
            .unwrap_err(),
        ImportFailure::Rubrica(RubricaError::GroupNotFound)
    ));
    assert_eq!(
        snapshot(world.root.path()),
        before,
        "a refusal writes nothing"
    );
    // and the preview is still there for the next try
    let summary = imports
        .commit(preview.id, "  Good  ", Some("Clienti"))
        .unwrap();
    assert_eq!(summary.name, "Good");
    assert_eq!(summary.group.as_deref(), Some("Clienti"));
}

#[test]
fn suggested_name_avoids_the_names_in_use() {
    let world = World::new();
    let imports = world.imports();
    for (expected, saved_as) in [
        ("inline", "inline"),
        ("inline (2)", "inline (2)"),
        ("inline (3)", "Other"),
        ("inline (3)", "Other 2"),
    ] {
        let preview = imports
            .offer_path(&world.files.inline, Origin::Picker)
            .unwrap();
        assert_eq!(preview.suggested_name, expected);
        imports.commit(preview.id, saved_as, None).unwrap();
    }
    // the text of a profile has no name of its own: the page says it
    let text = fs::read_to_string(&world.files.inline).unwrap();
    assert_eq!(imports.offer_text(&text).unwrap().suggested_name, "");
    // a long name and a name with a control character stay names
    let long = world.root.path().join(format!("{}.ovpn", "n".repeat(120)));
    fs::copy(&world.files.inline, &long).unwrap();
    let first = imports.offer_path(&long, Origin::Drop).unwrap();
    assert_eq!(first.suggested_name.chars().count(), 64);
    imports
        .commit(first.id, &first.suggested_name, None)
        .unwrap();
    let second = imports.offer_path(&long, Origin::Drop).unwrap();
    assert_eq!(second.suggested_name.chars().count(), 64);
    assert!(second.suggested_name.ends_with(" (2)"));
}

// ---- how long a preview waits ---------------------------------------------------------------------------------------

#[test]
fn a_preview_expires_after_ten_minutes() {
    let world = World::new();
    let (imports, now) = world.imports_with_clock();
    let early = imports
        .offer_path(&world.files.inline, Origin::Picker)
        .unwrap();
    let late = imports
        .offer_path(&world.files.separate, Origin::Picker)
        .unwrap();
    *now.lock().unwrap() += PREVIEW_TTL - Duration::from_secs(1);
    // a minute less than ten: it is still good
    imports.commit(late.id, "Late", None).unwrap();
    *now.lock().unwrap() += Duration::from_secs(1);
    let error = error_of(imports.commit(early.id, "Early", None).unwrap_err());
    assert_eq!(error["code"], "gui_import_expired");
    assert_eq!(world.rubrica.list().unwrap().profiles.len(), 1);
}

#[test]
fn the_fifth_preview_drops_the_oldest() {
    let world = World::new();
    let imports = world.imports();
    let ids: Vec<Uuid> = (0..=MAX_PREVIEWS)
        .map(|_| {
            imports
                .offer_path(&world.files.inline, Origin::Picker)
                .unwrap()
                .id
        })
        .collect();
    assert_eq!(ids.len(), MAX_PREVIEWS + 1);
    assert!(matches!(
        imports.commit(ids[0], "x", None).unwrap_err(),
        ImportFailure::Expired
    ));
    for id in &ids[1..] {
        imports.commit(*id, "x", None).unwrap();
    }
}

#[test]
fn commit_and_discard_consume_the_preview() {
    let world = World::new();
    let imports = world.imports();
    let preview = imports
        .offer_path(&world.files.inline, Origin::Picker)
        .unwrap();
    imports.commit(preview.id, "Once", None).unwrap();
    for _ in 0..2 {
        assert!(matches!(
            imports.commit(preview.id, "Twice", None).unwrap_err(),
            ImportFailure::Expired
        ));
    }
    assert_eq!(world.rubrica.list().unwrap().profiles.len(), 1);

    let other = imports
        .offer_path(&world.files.separate, Origin::Picker)
        .unwrap();
    imports.discard(other.id);
    imports.discard(other.id);
    imports.discard(Uuid::new_v4());
    assert!(matches!(
        imports.commit(other.id, "Never", None).unwrap_err(),
        ImportFailure::Expired
    ));
    let error = error_of(ImportFailure::Expired);
    assert_eq!(error["code"], "gui_import_expired");
    assert_eq!(world.rubrica.list().unwrap().profiles.len(), 1);
}

// ---- the commands ---------------------------------------------------------------------------------------------------

struct Gui {
    _app: tauri::App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
}

/// The window «Open» of the tests: it answers with the file it is given, or cancels, and counts how often it was asked.
struct FakeFilePicker {
    answer: Option<PathBuf>,
    asked: Mutex<usize>,
}

impl FakeFilePicker {
    fn new(answer: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            answer,
            asked: Mutex::new(0),
        })
    }
}

impl FilePicker for FakeFilePicker {
    fn pick(&self) -> Option<PathBuf> {
        *self.asked.lock().unwrap() += 1;
        self.answer.clone()
    }
}

/// The app of the tests of level 2, with the rubrica of the world and **no daemon**: nothing here talks to one. The window
/// «Open» is the one the test gives (`None`: it is cancelled).
fn gui_picking(world: &World, picked: Option<PathBuf>) -> (Gui, Arc<FakeFilePicker>) {
    let picker = FakeFilePicker::new(picked);
    let dynamic: Arc<dyn FilePicker> = picker.clone();
    let gui = build_gui(
        world,
        mock_builder()
            .manage(Arc::clone(&world.rubrica))
            .manage(dynamic),
    );
    (gui, picker)
}

fn gui(world: &World) -> Gui {
    build_gui(world, mock_builder().manage(Arc::clone(&world.rubrica)))
}

fn build_gui(world: &World, builder: tauri::Builder<MockRuntime>) -> Gui {
    let mut app = configure_with(
        builder,
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
fn import_works_without_any_daemon() {
    let world = World::seeded();
    let gui = gui(&world);
    let text = fs::read_to_string(&world.files.malicious).unwrap();
    let preview = invoke(&gui.window, "import_from_text", json!({ "text": text })).unwrap();
    assert_eq!(preview["origin"], "text");
    assert_eq!(preview["removed"], 5);
    assert_eq!(preview["protocol"], "openvpn");
    assert_clean(&preview, &[world.root.path()]);
    // nothing was written yet
    let listed = |gui: &Gui| invoke(&gui.window, "profiles_list", json!({})).unwrap();
    assert_eq!(listed(&gui)["profiles"].as_array().unwrap().len(), 5);

    let saved = invoke(
        &gui.window,
        "import_commit",
        json!({ "id": preview["id"], "name": "Malicious", "group": "Lavoro" }),
    )
    .unwrap();
    assert_eq!(saved["name"], "Malicious");
    assert_eq!(saved["group"], "Lavoro");
    assert_clean(&saved, &[world.root.path()]);
    assert_eq!(listed(&gui)["profiles"].as_array().unwrap().len(), 6);
    // a second try on the same preview: it is gone
    let error = invoke(
        &gui.window,
        "import_commit",
        json!({ "id": preview["id"], "name": "Again", "group": null }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "gui_import_expired");
    // discard answers with nothing, for a preview that is there and for one that is not
    let again = invoke(&gui.window, "import_from_text", json!({ "text": text })).unwrap();
    assert_eq!(
        invoke(&gui.window, "import_discard", json!({ "id": again["id"] })).unwrap(),
        Value::Null
    );
    assert_eq!(
        invoke(&gui.window, "import_discard", json!({ "id": again["id"] })).unwrap(),
        Value::Null
    );
    // the same errors as above, through the command
    let error = invoke(
        &gui.window,
        "import_from_text",
        json!({ "text": "client\nca ca.crt\n" }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "import_missing_file");
    assert_eq!(error["data"]["origin"], "text");
}

#[test]
fn the_imported_text_is_the_one_the_cli_would_import() {
    // the window and the CLI import the same bytes into the same profile (I6)
    let world = World::new();
    let gui = gui(&world);
    let text = fs::read_to_string(&world.files.inline).unwrap();
    let preview = invoke(&gui.window, "import_from_text", json!({ "text": text })).unwrap();
    let saved = invoke(
        &gui.window,
        "import_commit",
        json!({ "id": preview["id"], "name": "inline", "group": null }),
    )
    .unwrap();
    let detail = invoke(&gui.window, "profile_get", json!({ "id": saved["id"] })).unwrap();
    let cli = import_file(&world.files.inline, None).unwrap();
    assert_eq!(
        detail["openvpn"]["config"].as_str().unwrap(),
        cli.profile.openvpn.unwrap().config
    );
    // the detail has the certificates (they are public) but no secret and no key
    let shown = detail.to_string();
    assert!(
        !shown.contains(SECRET_MARK) && !shown.contains("PRIVATE KEY"),
        "{shown}"
    );
}

// ---- the window «Open» ----------------------------------------------------------------------------------------------

#[test]
fn pick_file_returns_the_preview_of_the_picked_file() {
    let world = World::seeded();
    let (gui, picker) = gui_picking(&world, Some(world.files.inline.clone()));
    let preview = invoke(&gui.window, "import_pick_file", json!({})).unwrap();
    assert_eq!(preview["origin"], "picker");
    assert_eq!(preview["file_name"], "inline.ovpn");
    assert_eq!(preview["suggested_name"], "inline");
    assert_eq!(*picker.asked.lock().unwrap(), 1);
    // it waits to be confirmed like any other: nothing was written
    let listed = invoke(&gui.window, "profiles_list", json!({})).unwrap();
    assert_eq!(listed["profiles"].as_array().unwrap().len(), 5);
    let saved = invoke(
        &gui.window,
        "import_commit",
        json!({ "id": preview["id"], "name": "inline", "group": null }),
    )
    .unwrap();
    assert_eq!(saved["name"], "inline");
}

#[test]
fn pick_file_cancelled_is_null_and_not_an_error() {
    let world = World::seeded();
    let before = snapshot(&world.dir());
    let (gui, picker) = gui_picking(&world, None);
    assert_eq!(
        invoke(&gui.window, "import_pick_file", json!({})).unwrap(),
        Value::Null
    );
    assert_eq!(*picker.asked.lock().unwrap(), 1);
    assert_eq!(snapshot(&world.dir()), before);
}

#[test]
fn the_response_never_holds_the_folder() {
    let world = World::seeded();
    let (gui, _) = gui_picking(&world, Some(world.files.separate.clone()));
    let preview = invoke(&gui.window, "import_pick_file", json!({})).unwrap();
    assert_eq!(preview["file_name"], "separate.ovpn");
    assert_clean(
        &preview,
        &[world.root.path(), world.files.separate.parent().unwrap()],
    );
    // nor does an error: the file that is not there is named, the folder is not
    let (gui, _) = gui_picking(
        &world,
        Some(world.files.separate.with_file_name("nothing.ovpn")),
    );
    let error = invoke(&gui.window, "import_pick_file", json!({})).unwrap_err();
    assert_clean(&error, &[world.root.path()]);
}

#[test]
fn a_picked_file_gets_the_same_checks_as_a_dropped_one() {
    let world = World::new();
    let fifo = world.files.separate.with_file_name("fifo.ovpn");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let (gui, _) = gui_picking(&world, Some(fifo.clone()));
    let picked = invoke(&gui.window, "import_pick_file", json!({})).unwrap_err();
    assert_eq!(picked["code"], "gui_import_unsupported_file");
    let dropped = error_of(offer_error(world.imports().offer_path(&fifo, Origin::Drop)));
    assert_eq!(picked["code"], dropped["code"]);
    // the file that points outside of its folder is refused the same way too
    let (gui, _) = gui_picking(&world, Some(world.files.outside.clone()));
    let error = invoke(&gui.window, "import_pick_file", json!({})).unwrap_err();
    assert_eq!(error["code"], "import_outside_folder");
}

// ---- the files that come from outside (T4) --------------------------------------------------------------------------

fn preview_of(offer: Option<ImportOffer>) -> ImportPreview {
    match offer {
        Some(ImportOffer::Preview(preview)) => preview,
        other => panic!("a preview was expected: {other:?}"),
    }
}

/// The code of the error that an offer carries.
fn failed_code(offer: Option<ImportOffer>) -> String {
    match offer {
        Some(ImportOffer::Failed { error, .. }) => error.code,
        other => panic!("an error was expected: {other:?}"),
    }
}

#[test]
fn a_dropped_file_becomes_a_pending_preview() {
    let world = World::seeded();
    let imports = world.imports();
    let before = snapshot(&world.dir());
    let offered = preview_of(
        imports.offer_from_outside(std::slice::from_ref(&world.files.office), Origin::Drop),
    );
    assert_eq!(offered.origin, Origin::Drop);
    assert_eq!(offered.file_name.as_deref(), Some("office.conf"));
    assert_eq!(offered.protocol, Protocol::Wireguard);
    // it waits for the page: the same preview, once
    let waiting = preview_of(imports.take_pending());
    assert_eq!(waiting.id, offered.id);
    assert!(imports.take_pending().is_none());
    // and nothing was written before «Import»
    assert_eq!(snapshot(&world.dir()), before);
    imports.commit(offered.id, "Office", None).unwrap();
    assert_eq!(world.rubrica.list().unwrap().profiles.len(), 6);
}

#[test]
fn two_dropped_files_are_refused_and_nothing_is_previewed() {
    let world = World::seeded();
    let imports = world.imports();
    let before = snapshot(&world.dir());
    let offer = imports.offer_from_outside(
        &[world.files.inline.clone(), world.files.office.clone()],
        Origin::Drop,
    );
    assert_eq!(failed_code(offer), "gui_import_many");
    // the error is what waits for the page, and no file was read: no preview is held
    assert_eq!(failed_code(imports.take_pending()), "gui_import_many");
    assert_eq!(snapshot(&world.dir()), before);
    // no files at all (a text was dragged): nothing is offered and nothing waits
    assert!(imports.offer_from_outside(&[], Origin::Drop).is_none());
    assert!(imports.take_pending().is_none());
}

#[test]
fn a_dropped_folder_or_fifo_is_refused() {
    let world = World::new();
    let fifo = world.files.separate.with_file_name("fifo.ovpn");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let folder = world.files.separate.parent().unwrap().join("folder.ovpn");
    fs::create_dir(&folder).unwrap();
    for path in [fifo, folder] {
        // a thread and a deadline: a FIFO that blocked the read would stop the test for good
        let (tx, rx) = std::sync::mpsc::channel();
        let imports = Imports::new(Arc::clone(&world.rubrica));
        std::thread::spawn(move || {
            let _ = tx.send(imports.offer_from_outside(&[path], Origin::Drop));
        });
        let offer = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the read did not wait for a writer");
        assert_eq!(failed_code(offer), "gui_import_unsupported_file");
    }
}

#[test]
fn an_offer_made_before_the_page_is_ready_is_not_lost() {
    let world = World::new();
    let imports = world.imports();
    // nobody asked yet: the offer waits, with the file name and no folder
    let failed =
        imports.offer_from_outside(std::slice::from_ref(&world.files.outside), Origin::Open);
    assert_eq!(failed_code(failed), "import_outside_folder");
    let waiting = imports.take_pending();
    let Some(ImportOffer::Failed { file_name, .. }) = &waiting else {
        panic!("the error was lost: {waiting:?}");
    };
    assert_eq!(file_name.as_deref(), Some("outside.ovpn"));
    // taken once
    assert!(imports.take_pending().is_none());
}

#[test]
fn a_new_offer_replaces_the_pending_one_and_discards_its_preview() {
    let world = World::new();
    let imports = world.imports();
    let first = preview_of(
        imports.offer_from_outside(std::slice::from_ref(&world.files.inline), Origin::Drop),
    );
    let second = preview_of(
        imports.offer_from_outside(std::slice::from_ref(&world.files.office), Origin::Open),
    );
    assert_ne!(first.id, second.id);
    // the one that waits is the second; the first preview is gone with its secrets
    assert_eq!(preview_of(imports.take_pending()).id, second.id);
    assert!(matches!(
        imports.commit(first.id, "First", None).unwrap_err(),
        ImportFailure::Expired
    ));
    imports.commit(second.id, "Second", None).unwrap();
    // an offer that fails replaces a waiting preview too
    let third = preview_of(
        imports.offer_from_outside(std::slice::from_ref(&world.files.inline), Origin::Drop),
    );
    let failed =
        imports.offer_from_outside(std::slice::from_ref(&world.files.outside), Origin::Drop);
    assert_eq!(failed_code(failed), "import_outside_folder");
    assert!(matches!(
        imports.commit(third.id, "Third", None).unwrap_err(),
        ImportFailure::Expired
    ));
}

#[test]
fn the_offer_payload_holds_no_path_and_no_secret() {
    let world = World::seeded();
    let imports = world.imports();
    let folders = [world.root.path(), world.files.separate.parent().unwrap()];
    for file in [
        &world.files.malicious,
        &world.files.office,
        &world.files.separate,
    ] {
        let offer = imports
            .offer_from_outside(std::slice::from_ref(file), Origin::Drop)
            .unwrap();
        assert_clean(&offer, &folders);
        assert!(
            serde_json::to_value(&offer)
                .unwrap()
                .get("preview")
                .is_some()
        );
    }
    for files in [
        vec![world.files.outside.clone()],
        vec![world.files.inline.clone(), world.files.office.clone()],
        vec![world.files.inline.with_file_name("nothing.ovpn")],
    ] {
        let offer = imports.offer_from_outside(&files, Origin::Open).unwrap();
        assert_clean(&offer, &folders);
        assert!(
            serde_json::to_value(&offer)
                .unwrap()
                .get("failed")
                .is_some()
        );
    }
}

#[test]
fn import_pending_gives_the_waiting_offer_once_through_the_command() {
    let world = World::new();
    let gui = gui(&world);
    assert_eq!(
        invoke(&gui.window, "import_pending", json!({})).unwrap(),
        Value::Null
    );
    let imports = gui._app.state::<Arc<Imports>>();
    imports.offer_from_outside(std::slice::from_ref(&world.files.inline), Origin::Open);
    let offer = invoke(&gui.window, "import_pending", json!({})).unwrap();
    assert_eq!(offer["preview"]["origin"], "open");
    assert_eq!(offer["preview"]["file_name"], "inline.ovpn");
    assert_clean(&offer, &[world.root.path()]);
    assert_eq!(
        invoke(&gui.window, "import_pending", json!({})).unwrap(),
        Value::Null
    );
    // the command takes nothing: no path, no text
    imports.offer_from_outside(std::slice::from_ref(&world.files.inline), Origin::Open);
    assert!(invoke(&gui.window, "import_pending", json!({})).unwrap()["preview"].is_object());
}

#[test]
fn the_offer_goes_to_the_page_as_the_open_file_event() {
    let world = World::new();
    let gui = gui(&world);
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let sink = Arc::clone(&heard);
    gui._app
        .handle()
        .listen(galleria_gui::events::EV_OPEN_FILE, move |event| {
            sink.lock().unwrap().push(event.payload().to_owned());
        });
    // what the handler of the drop does, off the thread of the window
    galleria_gui::events::offer_from_outside(
        gui._app.handle(),
        std::slice::from_ref(&world.files.office),
        Origin::Drop,
    );
    // zero files: no event
    galleria_gui::events::offer_from_outside(gui._app.handle(), &[], Origin::Drop);
    let heard = heard.lock().unwrap();
    assert_eq!(heard.len(), 1, "{heard:?}");
    let payload: Value = serde_json::from_str(&heard[0]).unwrap();
    assert_eq!(payload["preview"]["origin"], "drop");
    assert_eq!(payload["preview"]["file_name"], "office.conf");
    assert_clean(&payload, &[world.root.path()]);
}

// ---- the whole round (T7) -------------------------------------------------------------------------------------------

#[test]
fn no_import_command_returns_a_secret_after_a_full_round() {
    // every way a file reaches the window, and every way it ends: nothing that comes back through a command, or that the
    // page hears as an event, holds a secret, a key or the folder of a file (I3)
    let world = World::seeded();
    let (gui, _picker) = gui_picking(&world, Some(world.files.separate.clone()));
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    for event in [
        galleria_gui::events::EV_OPEN_FILE,
        galleria_gui::events::EV_DROP_HOVER,
    ] {
        let sink = Arc::clone(&heard);
        gui._app.handle().listen(event, move |e| {
            sink.lock().unwrap().push(e.payload().to_owned());
        });
    }
    let mut seen: Vec<(&str, String)> = vec![];
    let mut note = |what: &'static str, answer: Result<Value, Value>| {
        let value = answer.unwrap_or_else(|e| e);
        seen.push((what, value.to_string()));
        value
    };
    let malicious = fs::read_to_string(&world.files.malicious).unwrap();
    let inline = fs::read_to_string(&world.files.inline).unwrap();

    // pasted text, saved
    let text = note(
        "import_from_text",
        invoke(
            &gui.window,
            "import_from_text",
            json!({ "text": malicious }),
        ),
    );
    note(
        "import_commit (text)",
        invoke(
            &gui.window,
            "import_commit",
            json!({ "id": text["id"], "name": "Text", "group": null }),
        ),
    );
    // a file chosen in the window «Open»: first given up, then saved
    let picked = note(
        "import_pick_file",
        invoke(&gui.window, "import_pick_file", json!({})),
    );
    note(
        "import_discard (picked)",
        invoke(&gui.window, "import_discard", json!({ "id": picked["id"] })),
    );
    let picked = note(
        "import_pick_file (again)",
        invoke(&gui.window, "import_pick_file", json!({})),
    );
    note(
        "import_commit (picked)",
        invoke(
            &gui.window,
            "import_commit",
            json!({ "id": picked["id"], "name": "Picked", "group": "Lavoro" }),
        ),
    );
    // a file dragged onto the window, saved; and one opened from outside, given up
    galleria_gui::events::offer_from_outside(
        gui._app.handle(),
        std::slice::from_ref(&world.files.office),
        Origin::Drop,
    );
    let dropped = note(
        "import_pending (drop)",
        invoke(&gui.window, "import_pending", json!({})),
    );
    note(
        "import_commit (drop)",
        invoke(
            &gui.window,
            "import_commit",
            json!({ "id": dropped["preview"]["id"], "name": "Dropped", "group": null }),
        ),
    );
    galleria_gui::events::offer_from_outside(
        gui._app.handle(),
        std::slice::from_ref(&world.files.inline),
        Origin::Open,
    );
    let opened = note(
        "import_pending (open)",
        invoke(&gui.window, "import_pending", json!({})),
    );
    note(
        "import_discard (open)",
        invoke(
            &gui.window,
            "import_discard",
            json!({ "id": opened["preview"]["id"] }),
        ),
    );
    // the errors: a text with a secret in it that is not a profile, a file outside its folder, two files, a preview that is gone
    note(
        "import_from_text (broken)",
        invoke(
            &gui.window,
            "import_from_text",
            json!({ "text": format!("client\n<key>\n{SECRET_MARK}broken\n") }),
        ),
    );
    note(
        "import_from_text (missing file)",
        invoke(
            &gui.window,
            "import_from_text",
            json!({ "text": "client\nca missing.crt\n" }),
        ),
    );
    galleria_gui::events::offer_from_outside(
        gui._app.handle(),
        std::slice::from_ref(&world.files.outside),
        Origin::Open,
    );
    note(
        "import_pending (outside folder)",
        invoke(&gui.window, "import_pending", json!({})),
    );
    galleria_gui::events::offer_from_outside(
        gui._app.handle(),
        &[world.files.inline.clone(), world.files.office.clone()],
        Origin::Drop,
    );
    note(
        "import_pending (two files)",
        invoke(&gui.window, "import_pending", json!({})),
    );
    note(
        "import_commit (gone)",
        invoke(
            &gui.window,
            "import_commit",
            json!({ "id": text["id"], "name": "Again", "group": null }),
        ),
    );
    let again = note(
        "import_from_text (to refuse the name)",
        invoke(&gui.window, "import_from_text", json!({ "text": inline })),
    );
    note(
        "import_commit (name refused)",
        invoke(
            &gui.window,
            "import_commit",
            json!({ "id": again["id"], "name": "   ", "group": null }),
        ),
    );
    note(
        "import_discard (last)",
        invoke(&gui.window, "import_discard", json!({ "id": again["id"] })),
    );
    // and what the window shows afterwards: the list and one imported profile
    let list = note(
        "profiles_list",
        invoke(&gui.window, "profiles_list", json!({})),
    );
    assert_eq!(list["profiles"].as_array().unwrap().len(), 8);
    note(
        "profile_get",
        invoke(
            &gui.window,
            "profile_get",
            json!({ "id": list["profiles"][0]["id"] }),
        ),
    );

    assert!(
        seen.iter().all(|(_, shown)| shown.len() > 2),
        "a command answered nothing: {seen:?}"
    );
    let folder = world.files.inline.parent().unwrap().to_string_lossy();
    let root = world.root.path().to_string_lossy();
    let events = heard.lock().unwrap();
    assert!(events.len() >= 4, "the page heard too little: {events:?}");
    for shown in seen
        .iter()
        .map(|(what, shown)| (*what, shown.as_str()))
        .chain(events.iter().map(|shown| ("an event", shown.as_str())))
    {
        let (what, shown) = shown;
        assert!(
            !shown.contains(SECRET_MARK),
            "{what}: a secret is in {shown}"
        );
        assert!(
            !shown.contains("PRIVATE KEY"),
            "{what}: a key is in {shown}"
        );
        assert!(
            !shown.contains(&*folder),
            "{what}: the folder is in {shown}"
        );
        assert!(!shown.contains(&*root), "{what}: the folder is in {shown}");
    }
}

// ---- the fixtures ---------------------------------------------------------------------------------------------------

#[test]
fn fixture_matches_the_lab_malicious_profile() {
    let script = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lab/server/make-profiles.py"),
    )
    .unwrap();
    let line = script
        .lines()
        .find(|l| l.contains("name==\"malicious\""))
        .expect("the lab has a malicious profile");
    let literal = line.split_once("text+=\"").unwrap().1;
    let literal = literal.strip_suffix('"').unwrap().replace("\\n", "\n");
    let lines: Vec<&str> = literal.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines.len() >= 8, "{lines:?}");
    let ours = fixtures::malicious_ovpn();
    let ours: Vec<&str> = ours.lines().collect();
    for line in lines {
        assert!(ours.contains(&line), "the fixture lacks {line:?}");
    }
}

#[test]
fn the_script_directives_are_the_same_in_rust_and_in_the_page() {
    let page = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/rubrica/scripts.ts"),
    )
    .unwrap();
    let list = page
        .split("SCRIPT_DIRECTIVES: ReadonlySet<string> = new Set([")
        .nth(1)
        .expect("scripts.ts lists the directives")
        .split("]);")
        .next()
        .unwrap();
    let mut page: Vec<&str> = list
        .split(',')
        .map(|name| name.trim().trim_matches('"'))
        .filter(|name| !name.is_empty())
        .collect();
    let mut rust: Vec<&str> = galleria_gui::import::SCRIPT_DIRECTIVES.to_vec();
    page.sort_unstable();
    rust.sort_unstable();
    assert_eq!(page, rust);
}
