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
    "allow-profile-validate-raw",
    "allow-profile-duplicate",
    "allow-profile-export",
    "allow-profile-delete",
    "allow-group-create",
    "allow-group-rename",
    "allow-group-delete",
    "allow-groups-reorder",
    "allow-import-from-text",
    "allow-import-commit",
    "allow-import-discard",
    "allow-import-pick-file",
    "allow-import-pending",
    "allow-secret-store-status",
    "allow-connect-profile",
    "allow-disconnect",
    "allow-announce-phrases",
    "allow-provide-input",
    "allow-forget-credentials",
    "allow-logs-get",
    "allow-logs-export",
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

/// «Export» opens the window «Save» from Rust (`export_gtk.rs`): the webview is granted nothing of a dialog or file
/// plugin, and no such plugin is in the app (plan B3, T8, S22).
#[test]
fn the_webview_has_no_dialog_or_file_permission() {
    let cap = read_json("capabilities/main.json");
    for permission in cap["permissions"].as_array().unwrap() {
        let name = permission.as_str().unwrap();
        for forbidden in ["dialog", "fs:", "opener", "shell"] {
            assert!(
                !name.contains(forbidden),
                "{name} gives the webview a {forbidden} permission"
            );
        }
    }
    let manifest = std::fs::read_to_string(crate_file("Cargo.toml")).unwrap();
    assert!(
        !manifest.contains("tauri-plugin"),
        "a Tauri plugin is a dependency"
    );
}

/// The drag of a file is handled by Tauri, which gives the paths to Rust (plan B4, T4, Q38): the handler stays on, and
/// the protocol that would let the page open a file stays off.
#[test]
fn drag_drop_stays_on_and_the_asset_protocol_stays_off() {
    let conf = tauri_conf();
    for window in conf["app"]["windows"].as_array().unwrap() {
        assert_ne!(
            window["dragDropEnabled"],
            Value::Bool(false),
            "{window} turns the drag and drop of Tauri off"
        );
    }
    let asset = &conf["app"]["security"]["assetProtocol"];
    assert!(
        asset.is_null() || asset["enable"] == Value::Bool(false),
        "the asset protocol is on: {asset}"
    );
    let csp = conf["app"]["security"]["csp"].as_str().unwrap();
    assert!(!csp.contains("asset:"), "the CSP names asset: {csp}");
}

// ---- the association of the .ovpn files (plan B4, T5, D31, Q41) ---------------------------------------------------------

fn desktop_file() -> String {
    let path = crate_file("../../packaging/linux/io.github.zpeppoz.galleria.desktop");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The value of a key of the file `.desktop` (the first one, outside of comments).
fn desktop_key(text: &str, key: &str) -> Option<String> {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .map(|value| value.trim().to_owned())
}

#[test]
fn only_ovpn_is_associated() {
    let conf = tauri_conf();
    let associations = conf["bundle"]["fileAssociations"].as_array().unwrap();
    assert_eq!(associations.len(), 1, "{associations:?}");
    assert_eq!(associations[0]["ext"], serde_json::json!(["ovpn"]));
    // a `.conf` may be anything (a WireGuard profile, a daemon configuration): it is not associated (D31)
    let text = serde_json::to_string(&conf["bundle"]).unwrap();
    assert!(!text.contains("\"conf\""), "{text}");
    assert!(!desktop_file().contains("wireguard"));
}

#[test]
fn the_desktop_file_and_the_bundle_agree() {
    let conf = tauri_conf();
    let mime = conf["bundle"]["fileAssociations"][0]["mimeType"]
        .as_str()
        .unwrap();
    let file = desktop_file();
    assert_eq!(desktop_key(&file, "MimeType"), Some(format!("{mime};")));
    // one file per launch, and the command is the binary of the app
    let exec = desktop_key(&file, "Exec").unwrap();
    assert!(exec.starts_with("galleria-gui "), "{exec}");
    assert!(exec.split_whitespace().any(|word| word == "%f"), "{exec}");
    assert_eq!(desktop_key(&file, "Type").as_deref(), Some("Application"));
    // the name of the file is the identifier of the app
    let identifier = conf["identifier"].as_str().unwrap();
    assert!(
        crate_file(&format!("../../packaging/linux/{identifier}.desktop")).exists(),
        "there is no {identifier}.desktop"
    );
}

#[test]
fn bundle_stays_off() {
    // the packages are phase 4: the association is declared and tried with the file `.desktop`, not installed
    assert_eq!(tauri_conf()["bundle"]["active"], Value::Bool(false));
}

// ---- the gates of B5 (plan B5, T2): a gate has a job, and the job ends by checking that its proofs ran ----

fn repo_file(name: &str) -> PathBuf {
    crate_file("../..").join(name)
}

/// The gates of B5 that have at least one proof written so far. Each task that closes a gate adds it here in the same
/// commit as its first row in `lab/tests/gates-b5.tsv`; at the end of the block this list is G4, G5, G6, G7 and G9.
const GATES_WITH_PROOFS: &[&str] = &["G4", "G5", "G6", "G7", "G9"];
const ALL_GATES: &[&str] = &["G4", "G5", "G6", "G7", "G9"];

/// `(gate, job, kind, name)` of every row of the manifest.
fn gate_rows() -> Vec<[String; 4]> {
    let path = repo_file("lab/tests/gates-b5.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| {
            let cells: Vec<String> = l.split('\t').map(|c| c.trim().to_string()).collect();
            <[String; 4]>::try_from(cells)
                .unwrap_or_else(|c| panic!("a row of the manifest needs 4 cells: {c:?}"))
        })
        .collect()
}

/// The text of a job of `ci.yml`: from its name, at two spaces, to the next job.
fn ci_job(ci: &str, job: &str) -> String {
    let header = format!("  {job}:");
    let mut lines = ci.lines().skip_while(|l| *l != header);
    assert!(lines.next().is_some(), "ci.yml has no job {job}");
    // up to the next job: a key at two spaces (comments at two spaces do not count)
    let body: Vec<&str> = lines
        .take_while(|l| {
            !l.starts_with("  ")
                || l.starts_with("   ")
                || !l.trim_end().ends_with(':')
                || l.trim_start().starts_with('#')
        })
        .collect();
    body.join("\n")
}

fn every_rust_source(dir: &std::path::Path, out: &mut String) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "target") {
                every_rust_source(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
        }
    }
}

#[test]
fn every_b5_gate_has_a_job() {
    let rows = gate_rows();
    assert!(!rows.is_empty(), "the manifest of the gates is empty");
    for gate in GATES_WITH_PROOFS {
        assert!(
            rows.iter().any(|r| &r[0] == gate),
            "{gate} has no proof in lab/tests/gates-b5.tsv"
        );
    }
    for [gate, job, kind, name] in &rows {
        assert!(
            ALL_GATES.contains(&gate.as_str()),
            "{gate} is not a gate of B5 ({name})"
        );
        assert!(
            ["cargo", "vitest", "lab"].contains(&kind.as_str()),
            "{kind} is not a kind ({name})"
        );
        assert!(!job.is_empty() && !name.is_empty());
    }
    // every job of the manifest is in ci.yml, and its last step is the check of its own proofs
    let ci = std::fs::read_to_string(repo_file(".github/workflows/ci.yml")).unwrap();
    let mut jobs: Vec<&String> = rows.iter().map(|r| &r[1]).collect();
    jobs.sort();
    jobs.dedup();
    for job in jobs {
        let text = ci_job(&ci, job);
        let last_step = text
            .split("\n      - ")
            .last()
            .unwrap_or_default()
            .to_string();
        assert!(
            last_step.contains(&format!("check-gates.py --job {job}")),
            "the last step of the job {job} of ci.yml must call `scripts/check-gates.py --job {job}`, it is:\n{last_step}"
        );
    }
}

#[test]
fn every_cargo_proof_of_the_manifest_is_a_test_that_exists() {
    // a name with a typo would be MISSING in CI only after the push: here it fails before
    let mut sources = String::new();
    every_rust_source(&repo_file("crates"), &mut sources);
    for [gate, _, kind, name] in gate_rows() {
        if kind == "cargo" {
            assert!(
                sources.contains(&format!("fn {name}(")),
                "{gate}: no test function {name} in crates/"
            );
        }
    }
}
