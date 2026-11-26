//! Import in the window (plan B4, T1): the previews that wait in memory for «Import». No type of Tauri in here, and
//! nothing that is a secret in what leaves: a preview carries the **names** of the secrets it found (`secret_refs`),
//! never their values and never the text of the profile (spec §3.2, rules 1 and 2). The profile and its secrets stay
//! here, in memory, until `commit` or `discard`: nothing before `commit` opens the folder of the profiles for writing (M2).
//!
//! One point holds the previews, and every way in is a way of calling it (P1). A path is never trusted: it is read with
//! the checks of `galleria_profile::read_import_file` (a regular file, at most 1 MiB, UTF-8).

use crate::{
    error::CmdError,
    rubrica::{AuthFlags, ProfileSummary, Rubrica, RubricaError, openvpn_view, summarize},
};
use galleria_ipc::Protocol;
use galleria_profile::{
    IMPORT_MAX_BYTES, ImportError, ImportFileError, ImportWarning, Imported, WarningKind,
    detect_protocol, import_read_file, import_text, openvpn_text, read_import_file,
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// How long a preview waits (spec §4.2) and how many wait together: the page cannot fill the memory by asking for
/// previews in a loop.
pub const PREVIEW_TTL: Duration = Duration::from_secs(10 * 60);
pub const MAX_PREVIEWS: usize = 4;

/// Where the file or the text of an import came from.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Picker,
    Drop,
    Open,
    Text,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Dangerous,
    Unsupported,
    NoTlsClient,
}

/// One thing the import dropped or noticed: the line and the name of the directive, **never its arguments** (P3): an
/// argument may be a secret or a path of the machine of whoever wrote the profile. `dns_script` is the only thing the
/// page learns of the arguments: whether the script is one of the two that set the DNS of the server, which Galleria
/// does by itself (D19).
#[derive(Debug, Serialize, Clone)]
pub struct ImportNotice {
    pub line: usize,
    pub directive: String,
    pub kind: NoticeKind,
    pub dns_script: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImportPreview {
    pub id: Uuid,
    pub origin: Origin,
    /// The name of the file, never its folder.
    pub file_name: Option<String>,
    pub suggested_name: String,
    pub protocol: Protocol,
    pub server: Option<String>,
    pub auth: AuthFlags,
    /// The names of the secrets found (`secret_refs`).
    pub found: Vec<String>,
    pub notices: Vec<ImportNotice>,
    /// The directives that were dropped: the dangerous and the unsupported ones. A missing `client` is not one.
    pub removed: usize,
    pub groups: Vec<String>,
    pub expires_in_secs: u64,
}

/// Why an import did not give a preview or a profile. The message never has the text, a secret or a folder: what the
/// webview needs to say it is in the code and in the data of the error.
#[derive(Debug, thiserror::Error)]
pub enum ImportFailure {
    #[error("Invalid VPN profile at line {line}")]
    Syntax { line: usize },
    /// A file that the profile refers to cannot be read (always, for pasted text: it has no folder).
    #[error("A file that the profile refers to cannot be read")]
    MissingFile { file: String, origin: Origin },
    #[error("A file that the profile refers to is outside its folder")]
    OutsideFolder { file: String },
    #[error("Invalid VPN profile")]
    Invalid,
    #[error("Cannot read the file")]
    Unreadable { file_name: Option<String> },
    #[error("The file or the text is too large")]
    TooLarge,
    #[error("Not a .ovpn or .conf profile")]
    UnsupportedFile { file_name: Option<String> },
    #[error("Only one file can be imported at a time")]
    Many,
    #[error("The preview is no longer there")]
    Expired,
    #[error(transparent)]
    Rubrica(#[from] RubricaError),
}

impl ImportFailure {
    fn from_import(error: ImportError, origin: Origin) -> Self {
        match error {
            ImportError::Syntax(line) => Self::Syntax { line },
            ImportError::MissingFile(file) => Self::MissingFile { file, origin },
            ImportError::OutsideFolder(file) => Self::OutsideFolder { file },
            ImportError::Invalid => Self::Invalid,
        }
    }

    /// The import of the phase 1 says «invalid» for an OpenVPN text that does not parse; the parser knows the line, and the
    /// window says it (`import_syntax`). Anything else stays as it is.
    fn at_line_of(self, text: &str) -> Self {
        if !matches!(self, Self::Invalid) || detect_protocol(text) != Protocol::Openvpn {
            return self;
        }
        match openvpn_text::parse(text) {
            Err(
                openvpn_text::ParseError::Syntax { line }
                | openvpn_text::ParseError::UnterminatedBlock { line },
            ) => Self::Syntax { line },
            Ok(_) => self,
        }
    }

    fn from_file(error: ImportFileError, origin: Origin, file_name: Option<String>) -> Self {
        match error {
            ImportFileError::Unreadable(_) => Self::Unreadable { file_name },
            ImportFileError::NotAFile | ImportFileError::UnsupportedFile => {
                Self::UnsupportedFile { file_name }
            }
            ImportFileError::TooLarge => Self::TooLarge,
            ImportFileError::Import(error) => Self::from_import(error, origin),
        }
    }
}

/// What the page is told of a file that came from outside (dropped on the window, or given as an argument): the preview,
/// or the error with the name of the file (never its folder). The same type goes out as the payload of
/// `galleria:open-file` and as the answer of `import_pending`.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ImportOffer {
    Preview(ImportPreview),
    Failed {
        error: CmdError,
        file_name: Option<String>,
    },
}

impl ImportOffer {
    fn failed(failure: ImportFailure, path: Option<&Path>) -> Self {
        Self::Failed {
            error: CmdError::from(failure),
            file_name: path
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned()),
        }
    }
}

/// A preview that waits: the profile and its secrets as the import made them, and when it was made.
struct Held {
    id: Uuid,
    at: Instant,
    imported: Imported,
}

type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

pub struct Imports {
    rubrica: Arc<Rubrica>,
    clock: Clock,
    held: Mutex<Vec<Held>>,
    /// The last offer from outside that the page has not taken yet (the page may not be there yet).
    pending: Mutex<Option<ImportOffer>>,
}

/// The name of the file without its extension, with nothing that is not a name: no control character, at most 64
/// characters.
fn name_from_file(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
    stem.unwrap_or_default()
        .chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect::<String>()
        .trim()
        .chars()
        .take(crate::edit::NAME_MAX)
        .collect()
}

/// `base`, or `base (2)`, `base (3)`… the first that no profile has. The suffix is made to fit the longest name.
fn unused_name(base: &str, taken: &[String]) -> String {
    if base.is_empty() || !taken.iter().any(|n| n == base) {
        return base.to_owned();
    }
    for n in 2.. {
        let suffix = format!(" ({n})");
        let room = crate::edit::NAME_MAX - suffix.chars().count();
        let candidate = format!("{}{suffix}", base.chars().take(room).collect::<String>());
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("an unused number is always found")
}

/// Whether the script of a directive is one of the two that set the DNS of the server (P10): the first argument, without
/// its folder and without a `.sh`.
fn is_resolver_script(argument: &str) -> bool {
    let file = argument.rsplit('/').next().unwrap_or(argument);
    let file = file.strip_suffix(".sh").unwrap_or(file);
    matches!(file, "update-resolv-conf" | "update-systemd-resolved")
}

/// The directives of an OpenVPN profile that run a script or a plugin: the same list as `ui/src/rubrica/scripts.ts`, which
/// decides the text of D19 for them (`tests/import.rs` keeps the two equal).
pub const SCRIPT_DIRECTIVES: &[&str] = &[
    "up",
    "down",
    "route-up",
    "route-pre-down",
    "ipchange",
    "tls-verify",
    "client-connect",
    "client-disconnect",
    "learn-address",
    "auth-user-pass-verify",
    "plugin",
    "up-restart",
    "dns-updown",
];

/// Reads the text again (the warning has no arguments) and answers only yes or no: is the directive that was dropped
/// at this line a script that sets the DNS of the server, which Galleria does by itself?
fn dns_script(items: &[openvpn_text::Item], warning: &ImportWarning) -> bool {
    warning.kind == WarningKind::Dangerous
        && SCRIPT_DIRECTIVES.contains(&warning.directive.as_str())
        && items.iter().any(|item| {
            matches!(
                item,
                openvpn_text::Item::Directive { line, name, args }
                    if *line == warning.line
                        && *name == warning.directive
                        && args.first().is_some_and(|a| is_resolver_script(a))
            )
        })
}

fn notices(text: &str, protocol: Protocol, warnings: &[ImportWarning]) -> Vec<ImportNotice> {
    // only an OpenVPN text has a script that sets the DNS: a WireGuard hook is a command line
    let items = if protocol == Protocol::Openvpn {
        openvpn_text::parse(text).unwrap_or_default()
    } else {
        vec![]
    };
    warnings
        .iter()
        .map(|w| ImportNotice {
            line: w.line,
            directive: w.directive.clone(),
            kind: match w.kind {
                WarningKind::Dangerous => NoticeKind::Dangerous,
                WarningKind::Unsupported => NoticeKind::Unsupported,
                WarningKind::NoTlsClient => NoticeKind::NoTlsClient,
            },
            dns_script: dns_script(&items, w),
        })
        .collect()
}

impl Imports {
    pub fn new(rubrica: Arc<Rubrica>) -> Self {
        Self::with_clock(rubrica, Arc::new(Instant::now))
    }

    /// The clock comes from outside, so that the tests do not wait ten minutes.
    pub fn with_clock(rubrica: Arc<Rubrica>, clock: Clock) -> Self {
        Self {
            rubrica,
            clock,
            held: Mutex::new(vec![]),
            pending: Mutex::new(None),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Held>> {
        // a panic elsewhere does not make the previews unusable
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The previews that are still good: the old ones are let go here, which wipes their secrets.
    fn fresh<'a>(&self, held: &'a mut Vec<Held>) -> &'a mut Vec<Held> {
        let now = (self.clock)();
        held.retain(|h| now.saturating_duration_since(h.at) < PREVIEW_TTL);
        held
    }

    /// The preview of a text that the user pasted. The text has no folder: a reference to a file in it is an error that
    /// tells so.
    ///
    /// # Errors
    ///
    /// [`ImportFailure`]: too large, or a profile that does not import.
    pub fn offer_text(&self, text: &str) -> Result<ImportPreview, ImportFailure> {
        if text.len() as u64 > IMPORT_MAX_BYTES {
            return Err(ImportFailure::TooLarge);
        }
        let imported = import_text(text, "")
            .map_err(|e| ImportFailure::from_import(e, Origin::Text).at_line_of(text))?;
        self.keep(imported, text, Origin::Text, None, String::new())
    }

    /// The preview of a file. The path comes from outside and is not trusted: this is the only function that takes one.
    ///
    /// # Errors
    ///
    /// [`ImportFailure`]: the file is not a regular file of the right kind and size, or the profile does not import.
    pub fn offer_path(&self, path: &Path, origin: Origin) -> Result<ImportPreview, ImportFailure> {
        let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let fail = |e| ImportFailure::from_file(e, origin, file_name.clone());
        let text = read_import_file(path).map_err(fail)?;
        let imported = import_read_file(path, &text, Some(""))
            .map_err(fail)
            .map_err(|failure| failure.at_line_of(&text))?;
        let base = name_from_file(path);
        self.keep(imported, &text, origin, file_name.clone(), base)
    }

    /// The preview, or the error, of the files that came from outside (dropped, or given as arguments), kept for the page
    /// until it asks (`take_pending`). One file per gesture (Q40): more than one is an error that says so, and nothing is
    /// read. Without files there is nothing to offer (`None`). A new offer replaces the one that waits, and the preview of
    /// the old one is let go (P7). Every path is read as `offer_path` reads it: it is not trusted.
    pub fn offer_from_outside(&self, paths: &[PathBuf], origin: Origin) -> Option<ImportOffer> {
        let offer = match paths {
            [] => return None,
            [path] => match self.offer_path(path, origin) {
                Ok(preview) => ImportOffer::Preview(preview),
                Err(failure) => ImportOffer::failed(failure, Some(path)),
            },
            _ => ImportOffer::failed(ImportFailure::Many, None),
        };
        let replaced = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(offer.clone());
        if let Some(ImportOffer::Preview(old)) = replaced {
            self.discard(old.id);
        }
        Some(offer)
    }

    /// The offer that waits for the page, if there is one: the page takes it once.
    pub fn take_pending(&self) -> Option<ImportOffer> {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Puts the import among the previews and describes it.
    fn keep(
        &self,
        imported: Imported,
        text: &str,
        origin: Origin,
        file_name: Option<String>,
        base_name: String,
    ) -> Result<ImportPreview, ImportFailure> {
        let view = self.rubrica.list()?;
        let taken: Vec<String> = view.profiles.iter().map(|p| p.name.clone()).collect();
        let profile = &imported.profile;
        let (server, auth) = match &profile.openvpn {
            Some(ovpn) => {
                let view = openvpn_view(&ovpn.config, &profile.secret_refs);
                (summarize(profile, None).server, view.auth)
            }
            None => (summarize(profile, None).server, AuthFlags::default()),
        };
        let notices = notices(text, profile.protocol, &imported.warnings);
        let removed = notices
            .iter()
            .filter(|n| n.kind != NoticeKind::NoTlsClient)
            .count();
        let id = Uuid::new_v4();
        let preview = ImportPreview {
            id,
            origin,
            file_name,
            suggested_name: unused_name(&base_name, &taken),
            protocol: profile.protocol,
            server,
            auth,
            found: profile.secret_refs.clone(),
            notices,
            removed,
            groups: view.groups,
            expires_in_secs: PREVIEW_TTL.as_secs(),
        };
        let mut held = self.lock();
        let held = self.fresh(&mut held);
        held.push(Held {
            id,
            at: (self.clock)(),
            imported,
        });
        // the oldest goes first: the page cannot fill the memory
        while held.len() > MAX_PREVIEWS {
            held.remove(0);
        }
        Ok(preview)
    }

    /// Saves the preview as a profile named `name`, in `group`, and lets it go. A name or a group that is refused leaves
    /// the preview where it is, so that the window can ask again; so does a store that cannot be written.
    ///
    /// # Errors
    ///
    /// [`ImportFailure::Expired`] when the preview is not there any more (too old, discarded, already saved), the errors
    /// of [`Rubrica::commit_import`] otherwise.
    pub fn commit(
        &self,
        id: Uuid,
        name: &str,
        group: Option<&str>,
    ) -> Result<ProfileSummary, ImportFailure> {
        let mut held = self.lock();
        let held = self.fresh(&mut held);
        let Some(at) = held.iter().position(|h| h.id == id) else {
            return Err(ImportFailure::Expired);
        };
        let summary = self
            .rubrica
            .commit_import(&held[at].imported, name, group)?;
        held.remove(at);
        Ok(summary)
    }

    /// Lets the preview go, with its secrets. A preview that is not there any more is not an error.
    pub fn discard(&self, id: Uuid) {
        let mut held = self.lock();
        held.retain(|h| h.id != id);
    }
}
