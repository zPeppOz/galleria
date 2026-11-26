//! The check for a newer version (plan B6, T4, G11, D16): when to ask, what to take from the answer, what to show. No type of Tauri and no
//! network in here: the request is made by a [`Fetch`] (`update_soup` in the app, a fake in the tests).
//!
//! What comes from the network is not trusted. The request goes to one address written in this file; from the answer only a **version
//! number** is taken, read again with `semver`, and the address the page shows is built here from that number. No string of the answer
//! reaches the page, the disk or `stderr`.

use crate::ui_state::{UiStateFile, UpdateState};
use semver::Version;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// The only remote address of the window, written once: this repository.
macro_rules! repository {
    () => {
        "https://github.com/zPeppOz/galleria"
    };
}

/// The page of the latest release. It answers with a redirect whose `Location` has the number of the version, so there is no body to read
/// and no `User-Agent` to send (measured, plan B6).
pub const RELEASES_LATEST: &str = concat!(repository!(), "/releases/latest");
/// What a version looks like in the `Location` of that page, and in the address the page is shown (`{TAG}{version}`).
const TAG: &str = concat!(repository!(), "/releases/tag/v");
/// The longest number of a version that is read: a hundred characters is far more than any release has.
const LONGEST_VERSION: usize = 100;
/// At most one request in this time, whatever the answer was (P11).
pub const DAY: Duration = Duration::from_secs(24 * 60 * 60);
/// The time one request may take (S52).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Why a request gave no answer. Only the kind: no address and no text of the system, so nothing of the network reaches `stderr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    #[error("could not connect")]
    Network,
    #[error("no answer in time")]
    Timeout,
}

/// One `GET` that does not follow redirects: the status and the value of `Location`, and nothing else of the answer.
pub trait Fetch: Send + Sync {
    /// # Errors
    ///
    /// When there is no answer: the kind of the failure.
    fn latest(&self, url: &str) -> Result<(u16, Option<String>), FetchError>;
}

/// A version newer than the one that runs, and where to read about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub version: Version,
    pub url: String,
}

/// What the page gets (`update_view`, and the event `galleria:update`). `latest` and `url` are there only when [`notice`] gives something.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpdateView {
    pub current: String,
    pub latest: Option<String>,
    pub url: Option<String>,
    pub enabled: bool,
}

/// Only if the check is on, and a day has passed since the last request, whatever it was (a request that failed counts). A date that cannot
/// be read is no date.
#[must_use]
pub fn due(state: &UpdateState, now: OffsetDateTime) -> bool {
    if !state.check {
        return false;
    }
    let last = state
        .last_check
        .as_deref()
        .and_then(|text| OffsetDateTime::parse(text, &Rfc3339).ok());
    last.is_none_or(|last| now - last >= DAY)
}

/// The version from the answer of the page of the releases: the status of a redirect and a `Location` that is exactly
/// `https://github.com/zPeppOz/galleria/releases/tag/v<semver>`, stable and with nothing after the number. Everything else is `None`: no
/// version known is a normal answer (today the repository has no release), not an error. Gives back only the number, never a piece of what
/// was received.
#[must_use]
pub fn parse(status: u16, location: Option<&str>) -> Option<Version> {
    if !matches!(status, 301 | 302 | 303 | 307 | 308) {
        return None;
    }
    let number = location?.strip_prefix(TAG)?;
    parse_stable(number)
}

/// A number of version that is stable: three numbers and nothing else (no pre-release, no build, no path, no query).
pub fn parse_stable(text: &str) -> Option<Version> {
    if text.len() > LONGEST_VERSION || !text.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    Version::parse(text).ok()
}

/// What to show: `latest` is newer than `current`, and not the one the person asked not to be reminded of (a newer one is shown again).
/// Nothing when the check is off. `latest` is read again here: what is on the disk is not trusted either.
#[must_use]
pub fn notice(current: &Version, state: &UpdateState) -> Option<Notice> {
    if !state.check {
        return None;
    }
    let latest = parse_stable(state.latest.as_deref()?)?;
    if latest <= *current {
        return None;
    }
    if let Some(dismissed) = state.dismissed.as_deref().and_then(parse_stable)
        && latest <= dismissed
    {
        return None;
    }
    Some(Notice {
        url: format!("{TAG}{latest}"),
        version: latest,
    })
}

/// The address of the request, and whether a value of `GALLERIA_UPDATE_URL` was refused.
#[derive(Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub url: String,
    pub ignored: bool,
}

/// `GALLERIA_UPDATE_URL` (P13) replaces the address for the tests, and only if it is `http://127.0.0.1:<port>/…` with nothing odd in it:
/// the variable cannot send the request anywhere else. Any other value is ignored and the caller says so on `stderr`.
#[must_use]
pub fn endpoint(value: Option<&str>) -> Endpoint {
    let honoured = value.filter(|text| is_loopback(text));
    Endpoint {
        url: honoured.unwrap_or(RELEASES_LATEST).to_owned(),
        ignored: value.is_some() && honoured.is_none(),
    }
}

fn is_loopback(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("http://127.0.0.1:") else {
        return false;
    };
    let (port, path) = rest.split_once('/').unwrap_or((rest, ""));
    !port.is_empty()
        && port.len() <= 5
        && port.bytes().all(|b| b.is_ascii_digit())
        && rest.contains('/')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-._~".contains(&b))
}

type Clock = Arc<dyn Fn() -> OffsetDateTime + Send + Sync>;

/// Asks at most once a day, writes what it learned in `state.toml` and tells what is worth showing.
pub struct Updater {
    state: Arc<UiStateFile>,
    fetch: Arc<dyn Fetch>,
    url: String,
    current: Version,
    clock: Clock,
}

impl Updater {
    pub fn new(
        state: Arc<UiStateFile>,
        fetch: Arc<dyn Fetch>,
        url: String,
        current: Version,
        clock: Clock,
    ) -> Self {
        Self {
            state,
            fetch,
            url,
            current,
            clock,
        }
    }

    /// What the page shows now, from `state.toml`: no request.
    #[must_use]
    pub fn view(&self) -> UpdateView {
        let state = self.state.load().update;
        let found = notice(&self.current, &state);
        UpdateView {
            current: self.current.to_string(),
            latest: found.as_ref().map(|n| n.version.to_string()),
            url: found.map(|n| n.url),
            enabled: state.check,
        }
    }

    /// One look at the clock: asks if the check is on and a day has passed, and gives the notice if the answer was a newer version. At most
    /// one request, never a second one after a failure. `last_check` is written **before** asking, so that a request that never ends or a
    /// process that dies still counts; if it cannot be written nothing is asked.
    pub async fn tick(&self) -> Option<Notice> {
        let now = (self.clock)();
        if !due(&self.state.load().update, now) {
            return None;
        }
        let stamp = now.format(&Rfc3339).ok()?;
        if self
            .state
            .update(|s| s.update.last_check = Some(stamp))
            .is_err()
        {
            say("version check skipped: the state could not be written");
            return None;
        }
        let (fetch, url) = (Arc::clone(&self.fetch), self.url.clone());
        let answer = tokio::task::spawn_blocking(move || fetch.latest(&url))
            .await
            .unwrap_or(Err(FetchError::Network));
        match answer {
            Ok((status, location)) => {
                let latest = parse(status, location.as_deref()).map(|v| v.to_string());
                let state = self.state.update(|s| s.update.latest = latest).ok()?.update;
                notice(&self.current, &state)
            }
            Err(kind) => {
                // one line: the kind, no address (the page is told nothing)
                say(&format!("version check failed: {kind}"));
                None
            }
        }
    }
}

/// Where there is no libsoup: no request is made, and the check never finds anything.
pub struct NoFetch;

impl Fetch for NoFetch {
    fn latest(&self, _url: &str) -> Result<(u16, Option<String>), FetchError> {
        Err(FetchError::Network)
    }
}

/// One line on `stderr`. Written to the handle itself, not with `eprintln!`: the tests that read what the process wrote catch the file,
/// not the capture of the test harness.
fn say(line: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "galleria-gui: {line}");
}
