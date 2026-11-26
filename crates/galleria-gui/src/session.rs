//! The session of the window with the daemon (plan B5, T4): connect a profile of the rubrica, disconnect, and what follows the
//! state the daemon reports (the last use of a profile, the phrase the screen reader says). No type of Tauri in here.
//!
//! The parameters of a connection are a secret: they are made here, from the profile and the archive of the secrets, sent in
//! one call and dropped, which wipes them. Nothing in this module prints them, and no `Debug` is written for them.

use crate::{
    announce::{Urgency, clean},
    calls::{self, CallError},
    rubrica::{Rubrica, RubricaError},
    ui_state::UiStateFile,
};
use galleria_ipc::{
    InputKind, InputValues, ProfileRef, ProvideInputParams, Secret, SessionErrorCode, State, Status,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};
use time::OffsetDateTime;
use uuid::Uuid;

/// The longest phrase the page may give (it is read aloud as a whole).
const PHRASE_MAX: usize = 200;
const PHRASES_MAX: usize = 32;

/// The sentences of the status announcements in the language of the window (Q43). `{name}` is the only placeholder and
/// stands for the name of the profile.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Phrases {
    pub connected: String,
    pub disconnected: String,
    pub reconnecting: String,
    pub error: BTreeMap<String, String>,
    pub error_other: String,
}

impl Phrases {
    fn capped(mut self) -> Self {
        let cap = |text: &mut String| {
            if text.chars().count() > PHRASE_MAX {
                *text = text.chars().take(PHRASE_MAX).collect();
            }
        };
        cap(&mut self.connected);
        cap(&mut self.disconnected);
        cap(&mut self.reconnecting);
        cap(&mut self.error_other);
        let mut error = std::mem::take(&mut self.error);
        error.retain(|code, _| code.len() <= PHRASE_MAX);
        let mut kept = BTreeMap::new();
        for (code, mut text) in error.into_iter().take(PHRASES_MAX) {
            cap(&mut text);
            kept.insert(code, text);
        }
        self.error = kept;
        self
    }
}

/// Why a command of the session did not do what it was asked.
#[derive(Debug)]
pub enum SessionFailure {
    /// The rubrica or its archive of the secrets (a profile that is not there, a locked keyring, a key that is missing).
    Rubrica(RubricaError),
    /// The call to the daemon: an error of the daemon, or a daemon that cannot be reached.
    Call(CallError),
    /// The daemon has a session already. `foreign` is `false` when the profile is one of this rubrica (the window may then
    /// replace it on its own) and `true` when it is not, or when the rubrica cannot be read: when in doubt the window asks.
    AlreadyActive {
        profile: ProfileRef,
        foreign: bool,
        message: String,
    },
}

impl From<RubricaError> for SessionFailure {
    fn from(error: RubricaError) -> Self {
        Self::Rubrica(error)
    }
}

type Triple = (State, Option<String>, Option<SessionErrorCode>);

/// The user and the password of a connection that is on its way, held for «Remember» until the session of the daemon is `connected`
/// (spec §9, rule 4; plan B5, «Il viaggio di una credenziale», step 5). Nothing else is kept of what the user typed: not a code,
/// not a passphrase. `Debug` says nothing of the values, and `Secret` wipes them when they are dropped.
struct Pending {
    session: u64,
    profile: Uuid,
    username: Secret,
    password: Secret,
}

impl fmt::Debug for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pending(..)")
    }
}

/// The question the daemon asked last: what an answer is checked against before anything is held.
#[derive(Debug, Clone, Copy)]
struct Asked {
    session: u64,
    request: u64,
    kind: InputKind,
    profile: Option<Uuid>,
}

/// The session of the window. One for the app; it is shared between the commands and the thread that delivers the events.
pub struct Sessions {
    socket: PathBuf,
    rubrica: Arc<Rubrica>,
    state: Arc<UiStateFile>,
    phrases: Mutex<Option<Phrases>>,
    /// The last (state, profile, error code) that was looked at: what changes it is a fact to announce, what does not is not.
    last: Mutex<Triple>,
    /// The name of the last profile that was seen, for the sentence of a session that has gone.
    name: Mutex<String>,
    /// The session whose connection already touched the last use of its profile.
    touched: Mutex<Option<u64>>,
    /// The question the daemon is waiting on, as the events told it.
    asked: Mutex<Option<Asked>>,
    /// The credentials to remember when the session that took them is up.
    pending: Mutex<Option<Pending>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Sessions {
    pub fn new(socket: PathBuf, rubrica: Arc<Rubrica>, state: Arc<UiStateFile>) -> Self {
        Self {
            socket,
            rubrica,
            state,
            phrases: Mutex::new(None),
            last: Mutex::new((State::Disconnected, None, None)),
            name: Mutex::new(String::new()),
            touched: Mutex::new(None),
            asked: Mutex::new(None),
            pending: Mutex::new(None),
        }
    }

    /// Reads the profile and its secrets, makes the parameters of `connect` and sends them. They are dropped when the call
    /// ends, whatever its outcome. The state of the session is not in the answer: it arrives with the events.
    ///
    /// # Errors
    ///
    /// [`SessionFailure::AlreadyActive`] when the daemon has a session and `replace` is `false` (or one is being replaced),
    /// the errors of the rubrica for a profile that is not there, a key that is missing or a keyring that is locked, and
    /// the error of the call.
    pub async fn connect_profile(&self, id: Uuid, replace: bool) -> Result<(), SessionFailure> {
        // a connection that starts is not the one whose credentials were waiting to be remembered
        *lock(&self.pending) = None;
        let rubrica = Arc::clone(&self.rubrica);
        // The archive may be a keyring that waits for the user to unlock it: not on a thread of the runtime.
        let params = tokio::task::spawn_blocking(move || rubrica.connect_params(id, replace))
            .await
            .map_err(|_| SessionFailure::Call(CallError::Unreachable("the task ended".into())))??;
        let outcome = calls::connect(&self.socket, &params).await;
        drop(params);
        match outcome {
            Ok(_) => Ok(()),
            Err(CallError::Rpc {
                code,
                message,
                data,
            }) if code == "already_active" => {
                let profile = data
                    .as_ref()
                    .and_then(|data| data.get("profile"))
                    .and_then(|p| serde_json::from_value::<ProfileRef>(p.clone()).ok());
                match profile {
                    Some(profile) => Err(SessionFailure::AlreadyActive {
                        foreign: !self.is_own(&profile.id),
                        profile,
                        message,
                    }),
                    None => Err(SessionFailure::Call(CallError::Rpc {
                        code,
                        message,
                        data,
                    })),
                }
            }
            Err(error) => Err(SessionFailure::Call(error)),
        }
    }

    /// Asks the daemon to end the session. Nothing else disconnects: no other call of this module does.
    ///
    /// # Errors
    ///
    /// The error of the call; `not_active` is the daemon's way to say there was nothing to end.
    pub async fn disconnect(&self) -> Result<(), SessionFailure> {
        calls::disconnect(&self.socket)
            .await
            .map_err(SessionFailure::Call)
    }

    /// Gives the daemon the answer to its question. With `remember` and a question of credentials, the user and the password are held
    /// (and only they) until the session is `connected`; they are written then, and dropped at an error, a disconnect or another
    /// session. The values are consumed: the caller keeps no copy, and neither does this function beyond what was just said.
    ///
    /// # Errors
    ///
    /// The error of the call: `input_not_expected` for a question that expired or is not the one asked, or a daemon that cannot be
    /// reached. On an error nothing is held.
    pub async fn provide_input(
        &self,
        session: u64,
        request: u64,
        values: InputValues,
        remember: bool,
    ) -> Result<(), SessionFailure> {
        let held = remember && self.hold(session, request, &values);
        let outcome = calls::provide_input(
            &self.socket,
            ProvideInputParams {
                session,
                request,
                values,
            },
        )
        .await;
        if held && outcome.is_err() {
            let mut pending = lock(&self.pending);
            if pending.as_ref().is_some_and(|p| p.session == session) {
                *pending = None;
            }
        }
        outcome.map_err(SessionFailure::Call)
    }

    /// Holds the user and the password of an answer to a question of credentials. A code and a passphrase are never held; a question
    /// the window did not see, or one of a profile that is not in the rubrica, holds nothing. Held before the call is made, so that
    /// a `connected` that comes quicker than the answer finds it.
    fn hold(&self, session: u64, request: u64, values: &InputValues) -> bool {
        let asked = *lock(&self.asked);
        let Some(Asked {
            kind: InputKind::Credentials,
            profile: Some(profile),
            ..
        }) = asked.filter(|a| a.session == session && a.request == request)
        else {
            return false;
        };
        if values.otp.is_some()
            || values.key_passphrase.is_some()
            || !self.is_own(&profile.to_string())
        {
            return false;
        }
        let (Some(username), Some(password)) = (&values.username, &values.password) else {
            return false;
        };
        *lock(&self.pending) = Some(Pending {
            session,
            profile,
            username: Secret::new(username.clone()),
            password: password.clone(),
        });
        true
    }

    /// Whether credentials are waiting for the session to be up. The proofs ask; nothing else does.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        lock(&self.pending).is_some()
    }

    /// Takes the remembered user and password of a profile out of the archive and their names out of the profile (P6).
    ///
    /// # Errors
    ///
    /// The errors of the rubrica: a profile that is not there, a locked keyring.
    pub fn forget_credentials(&self, id: Uuid) -> Result<(), SessionFailure> {
        Ok(self.rubrica.forget_credentials(id)?)
    }

    /// Whether the session of the daemon belongs to a profile of this rubrica. A rubrica that cannot be read says no.
    fn is_own(&self, id: &str) -> bool {
        Uuid::parse_str(id).is_ok_and(|id| self.rubrica.get(id).is_ok())
    }

    /// The sentences of the announcements, in the language the window has now. Until they arrive nothing is announced.
    pub fn set_phrases(&self, phrases: Phrases) {
        *lock(&self.phrases) = Some(phrases.capped());
    }

    /// What follows a state the daemon reported, before the page gets it: the last use of the profile when a session of the
    /// rubrica is up, and the sentence to say when the (state, profile, error) changed.
    pub fn on_state(&self, status: &Status) -> Option<(String, Urgency)> {
        self.touch_last_use(status);
        self.follow_input(status);
        if let Some(profile) = &status.profile {
            lock(&self.name).clone_from(&profile.name);
        }
        let triple: Triple = (
            status.state,
            status.profile.as_ref().map(|p| p.id.clone()),
            status.error.as_ref().map(|e| e.code),
        );
        {
            let mut last = lock(&self.last);
            if *last == triple {
                return None;
            }
            *last = triple;
        }
        let phrases = lock(&self.phrases);
        let phrases = phrases.as_ref()?;
        let (template, urgency) = match status.state {
            State::Connected => (phrases.connected.as_str(), Urgency::Polite),
            State::Disconnected => (phrases.disconnected.as_str(), Urgency::Polite),
            State::Reconnecting => (phrases.reconnecting.as_str(), Urgency::Polite),
            State::Error => {
                let code = status
                    .error
                    .as_ref()
                    .and_then(|e| serde_json::to_value(e.code).ok())
                    .and_then(|v| v.as_str().map(str::to_owned));
                let text = code
                    .and_then(|code| phrases.error.get(&code))
                    .unwrap_or(&phrases.error_other);
                (text.as_str(), Urgency::Assertive)
            }
            // The window of the credentials takes the focus and says itself; the two in-between states are not facts.
            State::Connecting | State::NeedsInput | State::Disconnecting => return None,
        };
        let name = status
            .profile
            .as_ref()
            .map_or_else(|| lock(&self.name).clone(), |p| p.name.clone());
        let text = clean(&template.replace("{name}", &name))?;
        Some((text, urgency))
    }

    /// What the state says about the question and about the credentials that wait to be remembered: the question is noted, and the
    /// credentials are written when their session is `connected` and dropped when it ends or another one begins.
    fn follow_input(&self, status: &Status) {
        if let (State::NeedsInput, Some(request), Some(session)) =
            (status.state, &status.input_request, status.session)
        {
            *lock(&self.asked) = Some(Asked {
                session,
                request: request.request,
                kind: request.kind,
                profile: status
                    .profile
                    .as_ref()
                    .and_then(|p| Uuid::parse_str(&p.id).ok()),
            });
        }
        let taken = {
            let mut pending = lock(&self.pending);
            let Some(held) = pending.as_ref() else {
                return;
            };
            let ours = status.session.is_none_or(|s| s == held.session);
            match status.state {
                State::Connected if ours && status.session.is_some() => pending.take(),
                State::Connecting | State::NeedsInput | State::Reconnecting if ours => return,
                _ => {
                    *pending = None;
                    return;
                }
            }
        };
        if let Some(held) = taken {
            // not worth stopping anything for: the next connection asks again
            let _ = self
                .rubrica
                .remember_credentials(held.profile, &held.username, &held.password);
        }
    }

    /// «Last use» (P15): a session of a profile of the rubrica that reaches `connected` writes it, once per session. A session
    /// outside the rubrica writes nothing.
    fn touch_last_use(&self, status: &Status) {
        if status.state != State::Connected {
            return;
        }
        let (Some(session), Some(profile)) = (status.session, &status.profile) else {
            return;
        };
        {
            let mut touched = lock(&self.touched);
            if *touched == Some(session) {
                return;
            }
            *touched = Some(session);
        }
        let Ok(id) = Uuid::parse_str(&profile.id) else {
            return;
        };
        if self.rubrica.get(id).is_ok() {
            // a last use that cannot be written is not worth stopping anything for
            let _ = self.state.touch_last_used(id, OffsetDateTime::now_utc());
        }
    }
}
