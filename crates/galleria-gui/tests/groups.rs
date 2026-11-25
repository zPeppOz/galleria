//! The groups of the rubrica (plan B3, T6): created, renamed, deleted and ordered in `galleria.toml`, with the profiles
//! that belong to them. A group is a name (Q30): who is in it is `Profile.group`, the file holds the order and the empty
//! groups. No test here touches a secret, and none talks to a daemon.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;

use galleria_gui::{
    configure_with,
    rubrica::{Rubrica, RubricaError},
    supervisor::RelinkPolicy,
    ui_state::UiStateFile,
};
use galleria_profile::{ProfileStore, StoreError};
use rstest::rstest;
use seed::{GROUPS, SECRET_MARK, seed};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};

fn rubrica_in(root: &Path) -> (Rubrica, Arc<UiStateFile>) {
    let state = Arc::new(UiStateFile::new(root.join("state/galleria/state.toml")));
    (
        Rubrica::new(root.join("galleria"), Arc::clone(&state)),
        state,
    )
}

/// What the window would show: (name of the profile, its group) in the order of the list, and the groups.
fn shown(rubrica: &Rubrica) -> (Vec<(String, Option<String>)>, Vec<String>) {
    let view = rubrica.list().unwrap();
    let mut profiles: Vec<_> = view
        .profiles
        .iter()
        .map(|p| (p.name.clone(), p.group.clone()))
        .collect();
    profiles.sort();
    (profiles, view.groups)
}

fn group_of(rubrica: &Rubrica, name: &str) -> Option<String> {
    shown(rubrica)
        .0
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
        .1
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn create_appends_and_refuses_duplicates_ignoring_case() {
    let root = tempfile::tempdir().unwrap();
    seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());

    let list = rubrica.group_create("Famiglia").unwrap();
    assert_eq!(list, ["Lavoro", "Personale", "Clienti", "Famiglia"]);
    assert_eq!(rubrica.list().unwrap().groups, list, "it is in the file");
    for again in ["Famiglia", "famiglia", "FAMIGLIA", "  lavoro  "] {
        let error = rubrica.group_create(again).unwrap_err();
        assert!(
            matches!(error, RubricaError::GroupExists),
            "{again}: {error:?}"
        );
    }
    assert_eq!(rubrica.list().unwrap().groups.len(), 4, "nothing was added");
}

#[test]
fn a_group_that_only_the_profiles_have_can_be_added_to_the_list() {
    // the CLI or a file written by hand put a profile in a group that `galleria.toml` does not list (T4 shows it anyway)
    let root = tempfile::tempdir().unwrap();
    let galleria = root.path().join("galleria");
    let seeded = seed(&galleria, false);
    let store = ProfileStore::open(&galleria).unwrap();
    let mut profile = store.find(&seeded.profiles[0].1.to_string()).unwrap();
    profile.group = Some("Ospiti".into());
    store.save(&profile).unwrap();
    let (rubrica, _) = rubrica_in(root.path());
    assert_eq!(rubrica.list().unwrap().groups, GROUPS);

    let list = rubrica.group_create("Ospiti").unwrap();
    assert_eq!(list, ["Lavoro", "Personale", "Clienti", "Ospiti"]);
    assert_eq!(
        group_of(&rubrica, "Ufficio Napoli").as_deref(),
        Some("Ospiti")
    );
}

#[rstest]
#[case::trimmed("  Nuovo  ", Ok("Nuovo"))]
#[case::the_longest(
    "gggggggggggggggggggggggggggggggggggggggggggggggg",
    Ok("gggggggggggggggggggggggggggggggggggggggggggggggg")
)]
#[case::empty("", Err("required"))]
#[case::only_spaces("   ", Err("required"))]
#[case::too_long(&"g".repeat(49), Err("too_long"))]
#[case::a_newline("uno\ndue", Err("invalid_chars"))]
#[case::a_control_character("bell\u{7}", Err("invalid_chars"))]
fn names_are_trimmed_and_checked(#[case] name: &str, #[case] expected: Result<&str, &str>) {
    let root = tempfile::tempdir().unwrap();
    seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let result = rubrica.group_create(name);
    match (result, expected) {
        (Ok(list), Ok(stored)) => assert_eq!(list.last().map(String::as_str), Some(stored)),
        (Err(RubricaError::GroupInvalid(reason)), Err(code)) => assert_eq!(reason, code),
        (other, expected) => panic!("{name:?}: {other:?} instead of {expected:?}"),
    }
    // the rename follows the same rules (on a rubrica of its own: the name made above would clash with itself)
    let other = tempfile::tempdir().unwrap();
    seed(&other.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(other.path());
    let renamed = rubrica.group_rename("Clienti", name);
    match expected {
        Ok(stored) => assert_eq!(renamed.unwrap()[2], stored),
        Err(code) => assert!(
            matches!(renamed, Err(RubricaError::GroupInvalid(reason)) if reason == code),
            "{renamed:?}"
        ),
    }
}

#[test]
fn rename_moves_every_member_and_keeps_the_position() {
    let root = tempfile::tempdir().unwrap();
    seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());
    let before = rubrica.list().unwrap();
    let modified = |view: &galleria_gui::rubrica::RubricaView, name: &str| {
        view.profiles
            .iter()
            .find(|p| p.name == name)
            .unwrap()
            .modified
            .clone()
    };

    let list = rubrica.group_rename("Lavoro", "Ufficio").unwrap();
    assert_eq!(
        list,
        ["Ufficio", "Personale", "Clienti"],
        "the same place in the order"
    );
    let after = rubrica.list().unwrap();
    assert_eq!(after.groups, list);
    assert_eq!(
        group_of(&rubrica, "Ufficio Napoli").as_deref(),
        Some("Ufficio")
    );
    assert_eq!(
        group_of(&rubrica, "Datacenter Milano").as_deref(),
        Some("Ufficio")
    );
    assert_eq!(group_of(&rubrica, "Homelab").as_deref(), Some("Personale"));
    assert_eq!(
        group_of(&rubrica, "Cliente Rossi").as_deref(),
        Some("Clienti")
    );
    // the profiles that moved were written (a window that holds one learns it from `modified`), the others were not
    for moved in ["Ufficio Napoli", "Datacenter Milano"] {
        assert_ne!(modified(&after, moved), modified(&before, moved), "{moved}");
    }
    for same in ["Homelab", "VPS Hetzner", "Cliente Rossi"] {
        assert_eq!(modified(&after, same), modified(&before, same), "{same}");
    }

    // only the case of its own name: it is the same group, not a clash
    assert_eq!(
        rubrica.group_rename("Ufficio", "UFFICIO").unwrap()[0],
        "UFFICIO"
    );
    // the same name, and a name that another group has (whatever the case), and one that is not there
    assert_eq!(
        rubrica.group_rename("Personale", "Personale").unwrap()[1],
        "Personale"
    );
    for clash in ["Clienti", "clienti"] {
        let error = rubrica.group_rename("Personale", clash).unwrap_err();
        assert!(
            matches!(error, RubricaError::GroupExists),
            "{clash}: {error:?}"
        );
    }
    let error = rubrica.group_rename("Inesistente", "Altro").unwrap_err();
    assert!(matches!(error, RubricaError::GroupNotFound), "{error:?}");
}

#[test]
fn rename_is_resumable_after_a_failure() {
    // A rename writes the profiles first and the list last, so a failure in between leaves some profiles with the new name
    // while the list still has the old one. This is that state, made by hand; the order itself is held by
    // `rename_writes_the_profiles_before_the_list` and `delete_writes_the_profiles_before_the_list`. The same call finishes
    // the work and does not complain about the new name.
    let root = tempfile::tempdir().unwrap();
    let galleria = root.path().join("galleria");
    let seeded = seed(&galleria, false);
    let store = ProfileStore::open(&galleria).unwrap();
    let mut first = store.find(&seeded.profiles[0].1.to_string()).unwrap();
    assert_eq!(first.group.as_deref(), Some("Lavoro"));
    first.group = Some("Ufficio".into());
    store.save(&first).unwrap();
    let (rubrica, _) = rubrica_in(root.path());
    assert_eq!(
        rubrica.list().unwrap().groups,
        GROUPS,
        "the list was not written yet"
    );

    let list = rubrica.group_rename("Lavoro", "Ufficio").unwrap();
    assert_eq!(list, ["Ufficio", "Personale", "Clienti"]);
    assert_eq!(
        group_of(&rubrica, "Ufficio Napoli").as_deref(),
        Some("Ufficio")
    );
    assert_eq!(
        group_of(&rubrica, "Datacenter Milano").as_deref(),
        Some("Ufficio")
    );
    // and doing it again is harmless: «Lavoro» is not in the list any more
    assert!(matches!(
        rubrica.group_rename("Lavoro", "Ufficio"),
        Err(RubricaError::GroupNotFound)
    ));
}

// ---- the order of the writes -----------------------------------------------------------------------------------------
//
// «The profiles first, the list last» is what makes a rename or a delete that stops half way resumable. A failing write
// cannot be forced with permissions (the tests also run as root, where they do not stop anything), so the failure is the
// size of a file: the test runs itself again in a child process under `ulimit -f`, a limit that a profile fits in and
// the list of groups (made long on purpose) does not. The write of the list fails with the real error of the system and
// the child looks at what the failure left behind: with the order turned round the profiles would be as they were.

/// Set for the child process: the folder that holds the rubrica prepared by the parent.
const CHILD_ROOT: &str = "GALLERIA_TEST_FSIZE_ROOT";
/// The limit given to `ulimit -f`: 8 blocks, which are 4 KiB for `dash` and 8 KiB for `bash`.
const BLOCKS: usize = 8;

/// A rubrica whose list of groups is too long to be written under the limit, while its profiles are not.
fn rubrica_with_a_long_list(root: &Path) {
    let galleria = root.join("galleria");
    seed(&galleria, false);
    let store = ProfileStore::open(&galleria).unwrap();
    let mut settings = store.settings().unwrap();
    settings
        .groups
        .extend((0..300).map(|i| format!("Group number {i:03} {}", "x".repeat(24))));
    store.save_settings(&settings).unwrap();
    let size = |path: PathBuf| fs::metadata(path).unwrap().len();
    let longest_profile = fs::read_dir(galleria.join("profiles"))
        .unwrap()
        .map(|e| size(e.unwrap().path()))
        .max()
        .unwrap();
    assert!(
        longest_profile < 4096,
        "a profile ({longest_profile} bytes) must fit in the limit of either shell"
    );
    assert!(
        size(galleria.join("galleria.toml")) > 8192 + 512,
        "the list must not fit in the limit of either shell"
    );
}

/// Runs the test `name` again as a child that cannot write a file longer than the limit (and gets an error, not a
/// signal, when it tries). Fails with what the child said when the child fails.
fn run_under_the_file_limit(name: &str, root: &Path) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "trap '' XFSZ; ulimit -f {BLOCKS} && exec \"$0\" \"$@\""
        ))
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env(CHILD_ROOT, root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the child failed ({}):\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // the child really ran the test: `--exact` with a name that is not there would pass with zero tests
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "the child ran no test: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn group_of_profile(galleria: &Path, name: &str) -> Option<String> {
    let (profiles, _) = ProfileStore::open(galleria)
        .unwrap()
        .list_lenient()
        .unwrap();
    profiles.into_iter().find(|p| p.name == name).unwrap().group
}

#[test]
fn rename_writes_the_profiles_before_the_list() {
    let Some(root) = std::env::var_os(CHILD_ROOT).map(PathBuf::from) else {
        let root = tempfile::tempdir().unwrap();
        rubrica_with_a_long_list(root.path());
        return run_under_the_file_limit("rename_writes_the_profiles_before_the_list", root.path());
    };
    let galleria = root.join("galleria");
    let (rubrica, _) = rubrica_in(&root);
    let error = rubrica.group_rename("Lavoro", "Ufficio").unwrap_err();
    assert!(
        matches!(
            &error,
            RubricaError::Store {
                source: StoreError::Io(_),
                ..
            }
        ),
        "the list could not be written, so the call fails: {error:?}"
    );
    // the profiles were written before the list was tried...
    for name in ["Ufficio Napoli", "Datacenter Milano"] {
        assert_eq!(
            group_of_profile(&galleria, name).as_deref(),
            Some("Ufficio"),
            "{name}"
        );
    }
    // ...and the list is as it was, with the old name: the same call can finish the work
    let groups = ProfileStore::open(&galleria)
        .unwrap()
        .settings()
        .unwrap()
        .groups;
    assert_eq!(groups[0], "Lavoro");
    assert!(!groups.iter().any(|g| g == "Ufficio"));
}

#[test]
fn delete_writes_the_profiles_before_the_list() {
    let Some(root) = std::env::var_os(CHILD_ROOT).map(PathBuf::from) else {
        let root = tempfile::tempdir().unwrap();
        rubrica_with_a_long_list(root.path());
        return run_under_the_file_limit("delete_writes_the_profiles_before_the_list", root.path());
    };
    let galleria = root.join("galleria");
    let (rubrica, _) = rubrica_in(&root);
    let error = rubrica.group_delete("Lavoro").unwrap_err();
    assert!(
        matches!(
            &error,
            RubricaError::Store {
                source: StoreError::Io(_),
                ..
            }
        ),
        "the list could not be written, so the call fails: {error:?}"
    );
    for name in ["Ufficio Napoli", "Datacenter Milano"] {
        assert_eq!(group_of_profile(&galleria, name), None, "{name}");
    }
    let groups = ProfileStore::open(&galleria)
        .unwrap()
        .settings()
        .unwrap()
        .groups;
    assert_eq!(groups[0], "Lavoro", "the list still has the group");
}

#[test]
fn delete_leaves_the_profiles_without_a_group() {
    let root = tempfile::tempdir().unwrap();
    seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());

    let list = rubrica.group_delete("Lavoro").unwrap();
    assert_eq!(list, ["Personale", "Clienti"]);
    assert_eq!(group_of(&rubrica, "Ufficio Napoli"), None);
    assert_eq!(group_of(&rubrica, "Datacenter Milano"), None);
    assert_eq!(group_of(&rubrica, "Homelab").as_deref(), Some("Personale"));
    // the profiles are all still there
    assert_eq!(rubrica.list().unwrap().profiles.len(), 5);
    // an empty group goes away too, and one that is not there is not found
    rubrica.group_create("Vuoto").unwrap();
    assert_eq!(
        rubrica.group_delete("Vuoto").unwrap(),
        ["Personale", "Clienti"]
    );
    assert!(matches!(
        rubrica.group_delete("Vuoto"),
        Err(RubricaError::GroupNotFound)
    ));
}

#[test]
fn reorder_needs_a_permutation() {
    let root = tempfile::tempdir().unwrap();
    seed(&root.path().join("galleria"), false);
    let (rubrica, _) = rubrica_in(root.path());

    let list = rubrica
        .groups_reorder(&names(&["Clienti", "Lavoro", "Personale"]))
        .unwrap();
    assert_eq!(list, ["Clienti", "Lavoro", "Personale"]);
    assert_eq!(rubrica.list().unwrap().groups, list);
    for bad in [
        names(&["Clienti", "Lavoro"]),
        names(&["Clienti", "Lavoro", "Personale", "Altro"]),
        names(&["Clienti", "Clienti", "Lavoro"]),
        names(&["clienti", "lavoro", "personale"]),
        names(&[]),
    ] {
        let error = rubrica.groups_reorder(&bad).unwrap_err();
        assert!(
            matches!(error, RubricaError::GroupInvalid("not_a_permutation")),
            "{bad:?}: {error:?}"
        );
    }
    assert_eq!(
        rubrica.list().unwrap().groups,
        ["Clienti", "Lavoro", "Personale"]
    );
}

#[test]
fn group_commands_never_touch_the_secrets() {
    let root = tempfile::tempdir().unwrap();
    let galleria = root.path().join("galleria");
    seed(&galleria, false);
    let (rubrica, _) = rubrica_in(root.path());
    let read_all = || -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(galleria.join("secrets"))
            .unwrap()
            .map(|e| {
                let path = e.unwrap().path();
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect();
        files.sort();
        files
    };
    let before = read_all();
    assert!(
        before
            .iter()
            .any(|(_, bytes)| String::from_utf8_lossy(bytes).contains(SECRET_MARK)),
        "the seed has no secrets: the check below would prove nothing"
    );
    rubrica.group_create("Nuovo").unwrap();
    rubrica.group_rename("Lavoro", "Ufficio").unwrap();
    rubrica
        .groups_reorder(&names(&["Nuovo", "Ufficio", "Clienti", "Personale"]))
        .unwrap();
    rubrica.group_delete("Personale").unwrap();
    assert_eq!(read_all(), before);
}

#[test]
fn the_files_stay_in_the_format_of_phase_1() {
    // what the CLI calls: open the folder and list. And the profiles the groups moved still read with `Profile::from_toml`.
    let root = tempfile::tempdir().unwrap();
    let galleria = root.path().join("galleria");
    seed(&galleria, false);
    let (rubrica, _) = rubrica_in(root.path());
    rubrica.group_create("Nuovo").unwrap();
    rubrica.group_rename("Lavoro", "Ufficio").unwrap();
    rubrica.group_delete("Clienti").unwrap();
    let store = ProfileStore::open(&galleria).unwrap();
    let listed = store.list().unwrap();
    assert_eq!(listed.len(), 5);
    assert_eq!(
        store.settings().unwrap().groups,
        ["Ufficio", "Personale", "Nuovo"]
    );
    use std::os::unix::fs::PermissionsExt;
    for entry in fs::read_dir(galleria.join("profiles")).unwrap() {
        let mode = fs::metadata(entry.unwrap().path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

// ---- the commands ----------------------------------------------------------------------------------------------------

fn app(
    rubrica: Rubrica,
    state: Arc<UiStateFile>,
) -> (tauri::App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let mut app = configure_with(
        mock_builder().manage(Arc::new(rubrica)),
        std::env::temp_dir().join("galleria-no-daemon-groups.sock"),
        RelinkPolicy {
            delays: vec![Duration::from_millis(20)],
        },
        UiStateFile::new(state.path().to_path_buf()),
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
fn the_group_commands_answer_with_the_new_list_and_say_why_they_refuse() {
    let root = tempfile::tempdir().unwrap();
    seed(&root.path().join("galleria"), false);
    let (rubrica, state) = rubrica_in(root.path());
    let (_app, window) = app(rubrica, state);

    assert_eq!(
        invoke(&window, "group_create", json!({ "name": "Nuovo" })).unwrap(),
        json!(["Lavoro", "Personale", "Clienti", "Nuovo"])
    );
    assert_eq!(
        invoke(
            &window,
            "group_rename",
            json!({ "from": "Nuovo", "to": "Famiglia" })
        )
        .unwrap(),
        json!(["Lavoro", "Personale", "Clienti", "Famiglia"])
    );
    assert_eq!(
        invoke(
            &window,
            "groups_reorder",
            json!({ "names": ["Famiglia", "Lavoro", "Personale", "Clienti"] })
        )
        .unwrap(),
        json!(["Famiglia", "Lavoro", "Personale", "Clienti"])
    );
    assert_eq!(
        invoke(&window, "group_delete", json!({ "name": "Famiglia" })).unwrap(),
        json!(["Lavoro", "Personale", "Clienti"])
    );
    // and the list that `profiles_list` gives is the same
    let list = invoke(&window, "profiles_list", json!({})).unwrap();
    assert_eq!(list["groups"], json!(["Lavoro", "Personale", "Clienti"]));

    let exists = invoke(&window, "group_create", json!({ "name": "lavoro" })).unwrap_err();
    assert_eq!(exists["code"], "gui_group_exists");
    let invalid = invoke(&window, "group_create", json!({ "name": "  " })).unwrap_err();
    assert_eq!(invalid["code"], "gui_group_invalid");
    assert_eq!(invalid["data"]["reason"], "required");
    let order = invoke(&window, "groups_reorder", json!({ "names": ["Lavoro"] })).unwrap_err();
    assert_eq!(order["code"], "gui_group_invalid");
    assert_eq!(order["data"]["reason"], "not_a_permutation");
    let missing = invoke(&window, "group_delete", json!({ "name": "Inesistente" })).unwrap_err();
    assert_eq!(missing["code"], "gui_group_not_found");
    let missing = invoke(
        &window,
        "group_rename",
        json!({ "from": "Inesistente", "to": "X" }),
    )
    .unwrap_err();
    assert_eq!(missing["code"], "gui_group_not_found");
}
