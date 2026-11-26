//! `state.toml` (D32): what the window remembers between runs. Language, theme, sort order, size of the window and
//! the last use of each profile. No type of Tauri in here.
//!
//! The file is a convenience: if it is lost nothing happens, so reading never fails (a missing, unreadable or
//! newer file gives the defaults) and an unknown key is tolerated and dropped at the next write.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Mutex,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub const STATE_SCHEMA: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LangChoice {
    #[default]
    System,
    En,
    It,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Name,
    Recent,
}

/// Size of the window in logical pixels and whether it was maximised (Q34: the position is not saved).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub maximized: bool,
}

/// Smallest and largest side a saved size may have: a corrupted file must not open a window nobody can use.
const MIN_SIDE: u32 = 400;
const MAX_SIDE: u32 = 16_000;

impl WindowSize {
    /// The size to apply at startup, or `None` when the saved one is not usable.
    pub fn usable(self) -> Option<Self> {
        let ok = |side: u32| (MIN_SIDE..=MAX_SIDE).contains(&side);
        (ok(self.width) && ok(self.height)).then_some(self)
    }

    /// What to save for the window as it is now. A maximised window fills the screen: its size is not the one to
    /// come back to, so the width and height of the previous record stay (or the current ones when there is none).
    pub fn after(previous: Option<Self>, width: u32, height: u32, maximized: bool) -> Self {
        match (maximized, previous) {
            (true, Some(before)) => Self {
                maximized: true,
                ..before
            },
            _ => Self {
                width,
                height,
                maximized,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct UiState {
    pub schema_version: u32,
    pub lang: LangChoice,
    pub theme: ThemeChoice,
    pub sort: SortOrder,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowSize>,
    /// RFC 3339 per profile; written by B5 when a connection starts.
    pub last_used: BTreeMap<Uuid, String>,
    /// The version check (plan B6, T4). Left out of the file while it is all defaults, so a file that never touched it is the one B5 wrote.
    #[serde(skip_serializing_if = "UpdateState::is_default")]
    pub update: UpdateState,
}

/// `[update]` of `state.toml` (P11): whether to look for a newer version, when it last asked (whatever came of it), the newest it found and
/// the one the person asked not to be reminded of. A file without the table, or with part of it, has the defaults for the rest, so
/// `schema_version` stays 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct UpdateState {
    /// On unless the person turns it off (D16: «disattivabile» means on by itself).
    pub check: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dismissed: Option<String>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            check: true,
            last_check: None,
            latest: None,
            dismissed: None,
        }
    }
}

impl UpdateState {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA,
            lang: LangChoice::default(),
            theme: ThemeChoice::default(),
            sort: SortOrder::default(),
            window: None,
            last_used: BTreeMap::new(),
            update: UpdateState::default(),
        }
    }
}

/// What the webview may change (spec §3.2, rule 2): not the window and not the last use, which only Rust writes. Of `[update]` it may turn
/// the check on and off and say which version it does not want to hear of again; `latest` and `last_check` are Rust's.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UiPatch {
    pub lang: Option<LangChoice>,
    pub theme: Option<ThemeChoice>,
    pub sort: Option<SortOrder>,
    pub update_check: Option<bool>,
    pub update_dismissed: Option<String>,
}

impl UiPatch {
    /// A version to dismiss is a stable `semver` number and nothing else: it is compared with the ones the check finds.
    ///
    /// # Errors
    ///
    /// When `update_dismissed` is not one.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.update_dismissed.as_deref() {
            Some(text) if crate::update::parse_stable(text).is_none() => Err("update_dismissed"),
            _ => Ok(()),
        }
    }

    pub fn apply(self, state: &mut UiState) {
        if let Some(lang) = self.lang {
            state.lang = lang;
        }
        if let Some(theme) = self.theme {
            state.theme = theme;
        }
        if let Some(sort) = self.sort {
            state.sort = sort;
        }
        if let Some(check) = self.update_check {
            state.update.check = check;
        }
        if let Some(dismissed) = self.update_dismissed {
            state.update.dismissed = Some(dismissed);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UiStateError {
    #[error("cannot write {}: {source}", .path.display())]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("cannot encode the state: {0}")]
    Encode(#[from] toml::ser::Error),
    #[error("cannot format the time: {0}")]
    Time(#[from] time::error::Format),
}

pub struct UiStateFile {
    path: PathBuf,
    /// One writer at a time: the window size is saved from another thread than the commands.
    writing: Mutex<()>,
}

/// `$XDG_STATE_HOME/galleria/state.toml`, or `$HOME/.local/state/galleria/state.toml`. Pure, so it can be tried
/// without touching the environment of the test process.
pub fn state_path(xdg_state_home: Option<OsString>, home: Option<OsString>) -> PathBuf {
    let base = match xdg_state_home.filter(|s| !s.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.unwrap_or_default()).join(".local/state"),
    };
    base.join("galleria/state.toml")
}

impl UiStateFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            writing: Mutex::new(()),
        }
    }

    pub fn default_path() -> PathBuf {
        state_path(std::env::var_os("XDG_STATE_HOME"), std::env::var_os("HOME"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Never an error: a file that is missing, unreadable, broken or of a newer schema gives the defaults, with one
    /// line on stderr for the last three.
    pub fn load(&self) -> UiState {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return UiState::default(),
            Err(e) => {
                eprintln!(
                    "galleria: cannot read {}: {e}; using the defaults",
                    self.path.display()
                );
                return UiState::default();
            }
        };
        match toml::from_str::<UiState>(&text) {
            Ok(state) if state.schema_version <= STATE_SCHEMA => state,
            Ok(state) => {
                eprintln!(
                    "galleria: {} has schema {}, newer than {STATE_SCHEMA}; using the defaults",
                    self.path.display(),
                    state.schema_version
                );
                UiState::default()
            }
            Err(e) => {
                eprintln!(
                    "galleria: cannot parse {}: {e}; using the defaults",
                    self.path.display()
                );
                UiState::default()
            }
        }
    }

    /// Reads, changes, and writes with a rename: the file is `0600` in a directory that is `0700` when we create it.
    ///
    /// # Errors
    ///
    /// When the directory or the file cannot be written. The old file stays as it was.
    pub fn update(&self, change: impl FnOnce(&mut UiState)) -> Result<UiState, UiStateError> {
        let _one_writer = self
            .writing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut state = self.load();
        change(&mut state);
        state.schema_version = STATE_SCHEMA;
        let text = toml::to_string(&state)?;
        self.write(&text).map_err(|source| UiStateError::Write {
            path: self.path.clone(),
            source,
        })?;
        Ok(state)
    }

    fn write(&self, text: &str) -> std::io::Result<()> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| std::io::Error::other("the state file has no directory"))?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        let mut file = tempfile::NamedTempFile::new_in(dir)?;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
        file.write_all(text.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|e| e.error)?;
        Ok(())
    }

    /// B5 calls this when a connection starts.
    ///
    /// # Errors
    ///
    /// As [`UiStateFile::update`].
    pub fn touch_last_used(&self, id: Uuid, now: OffsetDateTime) -> Result<(), UiStateError> {
        let stamp = now.format(&Rfc3339)?;
        self.update(|state| {
            state.last_used.insert(id, stamp);
        })
        .map(|_| ())
    }

    /// «Delete» (B3-T8) forgets the profile.
    ///
    /// # Errors
    ///
    /// As [`UiStateFile::update`].
    pub fn forget(&self, id: Uuid) -> Result<(), UiStateError> {
        self.update(|state| {
            state.last_used.remove(&id);
        })
        .map(|_| ())
    }
}
