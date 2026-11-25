//! G8: every code the daemon (or the GUI) can send has a text in both catalogues of the webview, and every
//! state a label. The catalogues are `ui/src/i18n/{en,it}.json`; these tests read them as plain files.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleria_gui::error::GUI_CODES;
use galleria_gui::link::LinkState;
use galleria_ipc::{ErrorCode, SessionErrorCode, State};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// One list of variants generates both the array and a `match` with no fallback arm: a variant added to the
/// enum does not compile until it is added here, and once it is here the catalogue tests ask for its text.
macro_rules! every_variant {
    ($all:ident, $exhaustive:ident, $ty:ty, [$($variant:path),+ $(,)?]) => {
        const $all: &[$ty] = &[$($variant),+];
        #[allow(dead_code)]
        fn $exhaustive(value: $ty) {
            match value {
                $($variant)|+ => {}
            }
        }
    };
}

every_variant!(
    ERROR_CODES,
    error_code_is_listed,
    ErrorCode,
    [
        ErrorCode::HelloRequired,
        ErrorCode::IncompatibleProtocol,
        ErrorCode::MessageTooLarge,
        ErrorCode::TooManyClients,
        ErrorCode::AlreadyActive,
        ErrorCode::NotActive,
        ErrorCode::InvalidConfig,
        ErrorCode::ProfileRejected,
        ErrorCode::BackendUnavailable,
        ErrorCode::InputNotExpected,
        ErrorCode::Internal,
    ]
);
every_variant!(
    SESSION_ERROR_CODES,
    session_error_code_is_listed,
    SessionErrorCode,
    [
        SessionErrorCode::AuthFailed,
        SessionErrorCode::InputTimeout,
        SessionErrorCode::ServerUnreachable,
        SessionErrorCode::TunnelSetupFailed,
        SessionErrorCode::NetSetupFailed,
        SessionErrorCode::DnsUnavailable,
        SessionErrorCode::UnsupportedChallenge,
        SessionErrorCode::BackendExited,
    ]
);
every_variant!(
    STATES,
    state_is_listed,
    State,
    [
        State::Disconnected,
        State::Connecting,
        State::NeedsInput,
        State::Connected,
        State::Reconnecting,
        State::Disconnecting,
        State::Error,
    ]
);
every_variant!(
    LINK_STATES,
    link_state_is_listed,
    LinkState,
    [
        LinkState::Connecting,
        LinkState::Connected,
        LinkState::DaemonNotRunning,
        LinkState::PermissionDenied,
        LinkState::IncompatibleProtocol,
    ]
);

fn repo_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn catalogue(lang: &str) -> Value {
    let path = repo_file(&format!("ui/src/i18n/{lang}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn both() -> [(&'static str, Value); 2] {
    [("en", catalogue("en")), ("it", catalogue("it"))]
}

/// The keys of `wanted` that the catalogue lacks or leaves empty.
fn missing(catalogue: &Value, wanted: &[String]) -> Vec<String> {
    wanted
        .iter()
        .filter(|key| {
            catalogue
                .get(key.as_str())
                .and_then(Value::as_str)
                .is_none_or(|text| text.trim().is_empty())
        })
        .cloned()
        .collect()
}

fn serialized<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .expect("serialised as a string")
        .to_string()
}

fn keys(prefix: &str, codes: impl IntoIterator<Item = String>) -> Vec<String> {
    codes
        .into_iter()
        .map(|code| format!("{prefix}.{code}"))
        .collect()
}

fn assert_all_present(wanted: &[String]) {
    for (lang, cat) in both() {
        assert_eq!(
            missing(&cat, wanted),
            Vec::<String>::new(),
            "missing in {lang}"
        );
    }
}

fn daemon_codes() -> Vec<String> {
    ERROR_CODES.iter().map(|c| c.as_str().to_string()).collect()
}

fn session_codes() -> Vec<String> {
    SESSION_ERROR_CODES.iter().map(serialized).collect()
}

#[test]
fn every_daemon_error_code_has_text_in_both_catalogues() {
    assert_all_present(&keys("error", daemon_codes()));
}

#[test]
fn every_session_error_code_has_text_in_both_catalogues() {
    assert_all_present(&keys("error", session_codes()));
}

#[test]
fn every_gui_code_has_text_in_both_catalogues() {
    assert_all_present(&keys("error", GUI_CODES.iter().map(|c| c.to_string())));
}

#[test]
fn every_state_and_link_state_has_a_label() {
    assert_all_present(&keys("state", STATES.iter().map(serialized)));
    assert_all_present(&keys("bar", LINK_STATES.iter().map(serialized)));
}

#[test]
fn no_error_key_is_left_without_a_code() {
    let known: BTreeSet<String> = keys("error", daemon_codes())
        .into_iter()
        .chain(keys("error", session_codes()))
        .chain(keys("error", GUI_CODES.iter().map(|c| c.to_string())))
        .collect();
    for (lang, cat) in both() {
        let orphans: Vec<&String> = cat
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| key.starts_with("error.") && !known.contains(*key))
            .collect();
        assert!(
            orphans.is_empty(),
            "{lang}: error keys without a code: {orphans:?}"
        );
    }
}

/// Every string literal that looks like a code of the GUI in the sources of the crate.
fn gui_code_literals() -> BTreeSet<String> {
    fn walk(dir: &std::path::Path, out: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                for piece in text.split('"').skip(1).step_by(2) {
                    let is_code = ["gui_", "import_"].iter().any(|p| piece.starts_with(p))
                        && piece.chars().all(|c| c.is_ascii_lowercase() || c == '_');
                    if is_code {
                        out.insert(piece.to_string());
                    }
                }
            }
        }
    }
    let mut found = BTreeSet::new();
    walk(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    found
}

#[test]
fn gui_code_literals_are_registered() {
    let registered: BTreeSet<String> = GUI_CODES.iter().map(|c| c.to_string()).collect();
    let unregistered: Vec<String> = gui_code_literals()
        .difference(&registered)
        .cloned()
        .collect();
    assert!(
        unregistered.is_empty(),
        "codes used in the sources but not in GUI_CODES (and so without a text): {unregistered:?}"
    );
}

#[test]
fn the_checker_reports_a_missing_code() {
    let mut cat = catalogue("en");
    cat.as_object_mut().unwrap().remove("error.internal");
    assert_eq!(
        missing(&cat, &keys("error", daemon_codes())),
        vec!["error.internal".to_string()]
    );
}
