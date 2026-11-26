/// The commands the webview may call. Declaring them here makes Tauri generate an `allow-<name>` permission for
/// each, to be listed in `capabilities/main.json`; `tests/config.rs` keeps the two lists equal.
const COMMANDS: &[&str] = &[
    "daemon_view",
    "daemon_retry",
    "app_info",
    "ui_state_get",
    "ui_state_set",
    "announce",
    "profiles_list",
    "profile_get",
    "profile_update",
    "profile_validate_raw",
    "profile_duplicate",
    "profile_export",
    "profile_delete",
    "group_create",
    "group_rename",
    "group_delete",
    "groups_reorder",
    "import_from_text",
    "import_commit",
    "import_discard",
    "import_pick_file",
    "import_pending",
    "secret_store_status",
    "connect_profile",
    "disconnect",
    "announce_phrases",
    "provide_input",
    "forget_credentials",
    "logs_get",
    "logs_export",
    "tray_labels",
];

fn main() {
    let app = tauri_build::AppManifest::new().commands(COMMANDS);
    if let Err(e) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(app)) {
        eprintln!("tauri-build failed: {e:#}");
        std::process::exit(1);
    }
}
