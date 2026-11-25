//! Static checks on the Tauri configuration files. They read the JSON files directly and never start Tauri.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;
use std::path::PathBuf;

/// Every permission the main window may hold. A permission added to `capabilities/main.json` must be added here
/// on purpose, in the same commit.
const ALLOWED: &[&str] = &[
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "allow-daemon-view",
    "allow-daemon-retry",
    "allow-app-info",
    "allow-ui-state-get",
    "allow-ui-state-set",
    "allow-announce",
    "allow-profiles-list",
    "allow-profile-get",
    "allow-profile-update",
];

fn crate_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
}

fn read_json(name: &str) -> Value {
    let path = crate_file(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn tauri_conf() -> Value {
    read_json("tauri.conf.json")
}

#[test]
fn csp_is_set_and_has_no_remote_host() {
    let conf = tauri_conf();
    let csp = conf["app"]["security"]["csp"]
        .as_str()
        .expect("app.security.csp must be a string");
    assert!(!csp.trim().is_empty(), "csp is empty");
    assert!(!csp.contains("https:"), "csp allows https: {csp}");
    assert!(!csp.contains('*'), "csp has a wildcard: {csp}");
    assert!(
        !csp.contains("unsafe-eval"),
        "csp allows unsafe-eval: {csp}"
    );
    for (at, _) in csp.match_indices("http") {
        assert!(
            csp[at..].starts_with("http://ipc.localhost"),
            "the only http host allowed is http://ipc.localhost: {csp}"
        );
    }
}

#[test]
fn csp_has_no_unsafe_inline() {
    let conf = tauri_conf();
    let csp = conf["app"]["security"]["csp"]
        .as_str()
        .expect("app.security.csp must be a string");
    // A component that needs inline styles is changed; the policy is not loosened (plan B2, vincoli globali).
    assert!(
        !csp.contains("unsafe-inline"),
        "csp allows unsafe-inline: {csp}"
    );
}

#[test]
fn global_tauri_is_off_and_prototype_is_frozen() {
    let conf = tauri_conf();
    assert_eq!(conf["app"]["withGlobalTauri"], Value::Bool(false));
    assert_eq!(
        conf["app"]["security"]["freezePrototype"],
        Value::Bool(true)
    );
}

#[test]
fn identifier_is_the_agreed_one() {
    assert_eq!(tauri_conf()["identifier"], "io.github.zpeppoz.galleria");
}

#[test]
fn main_capability_grants_only_listed_permissions() {
    let cap = read_json("capabilities/main.json");
    assert_eq!(cap["windows"], serde_json::json!(["main"]));
    let mut granted: Vec<String> = cap["permissions"]
        .as_array()
        .expect("permissions must be an array")
        .iter()
        .map(|p| {
            p.as_str()
                .expect("permission entries must be plain strings")
                .to_owned()
        })
        .collect();
    let mut allowed: Vec<String> = ALLOWED.iter().map(|s| (*s).to_owned()).collect();
    granted.sort();
    allowed.sort();
    assert_eq!(granted, allowed);
}

#[test]
fn no_plugin_is_configured() {
    let conf = tauri_conf();
    let plugins = &conf["plugins"];
    let empty = plugins.is_null() || plugins.as_object().is_some_and(|o| o.is_empty());
    assert!(empty, "plugins must be absent or empty: {plugins}");
}

/// The command names in the `COMMANDS` constant of `build.rs`, which it passes to `AppManifest::commands`.
fn commands_in_build_rs() -> Vec<String> {
    let text = std::fs::read_to_string(crate_file("build.rs")).unwrap();
    let after = text
        .split("const COMMANDS: &[&str] = &[")
        .nth(1)
        .expect("build.rs must declare `const COMMANDS: &[&str] = &[...]`");
    let list = after.split("];").next().unwrap();
    list.split(',')
        .map(|name| name.trim().trim_matches('"').to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn every_registered_command_has_a_permission() {
    let mut declared: Vec<String> = commands_in_build_rs()
        .iter()
        .map(|name| format!("allow-{}", name.replace('_', "-")))
        .collect();
    let mut permitted: Vec<String> = ALLOWED
        .iter()
        .filter(|p| !p.contains(':'))
        .map(|p| (*p).to_owned())
        .collect();
    declared.sort();
    permitted.sort();
    assert_eq!(declared, permitted);
}
