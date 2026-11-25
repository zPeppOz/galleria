//! The rubrica as the window sees it (plan B3): the profiles of the user with their groups, in reading. No type of
//! Tauri in here, and nothing that is a secret: the views carry the *names* of the secrets of a profile
//! (`secret_refs`), never their values (spec §3.2, rule 1). It reads the same store as the CLI, with the functions of
//! `galleria-profile`, and never talks to the daemon: the rubrica is usable while the service is stopped.

use crate::edit::apply;
pub use crate::edit::{FieldIssue, ProfilePatch, check_wireguard};
use crate::ui_state::UiStateFile;
use galleria_ipc::Protocol;
use galleria_ovpn::Item;
use galleria_profile::{Profile, ProfileStore, Settings, Skipped, StoreError, WireGuardProfile};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

/// One row of the list: what is needed to draw it and to search and sort it.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileSummary {
    pub id: Uuid,
    pub name: String,
    pub protocol: Protocol,
    pub group: Option<String>,
    pub favorite: bool,
    /// The first `remote` of an OpenVPN profile or the endpoint of the first peer of a WireGuard one, as `host:port`.
    pub server: Option<String>,
    /// RFC 3339, from `state.toml`: nobody writes it before B5.
    pub last_used: Option<String>,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Remote {
    pub host: String,
    pub port: Option<u16>,
    /// `udp` or `tcp`: from the `remote` itself, else from `proto`, else the default of OpenVPN.
    pub transport: Option<String>,
}

/// Which ways to authenticate the configuration asks for, from its directives, its blocks and the secrets it keeps.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AuthFlags {
    pub certificate: bool,
    pub user_pass: bool,
    pub otp: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenVpnView {
    pub config: String,
    pub remotes: Vec<Remote>,
    pub auth: AuthFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileDetail {
    #[serde(flatten)]
    pub summary: ProfileSummary,
    pub notes: String,
    pub tags: Vec<String>,
    pub created: String,
    pub settings: Settings,
    pub secret_refs: Vec<String>,
    pub openvpn: Option<OpenVpnView>,
    /// No keys in here: the private and the shared keys are in the archive of the secrets.
    pub wireguard: Option<WireGuardProfile>,
}

/// A file of the profiles folder that could not be read, shown by its name with a code of its own.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedView {
    pub file_name: String,
    /// `gui_profile_newer` (written by a newer version) or `gui_profile_invalid`.
    pub code: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct RubricaView {
    pub profiles: Vec<ProfileSummary>,
    pub groups: Vec<String>,
    pub skipped: Vec<SkippedView>,
}

/// Why the rubrica could not do what was asked: the store (with the folder it happened in, because the error of the
/// store does not always carry a path), a profile that changed on disk since the window read it, or fields that may
/// not be saved.
#[derive(Debug, thiserror::Error)]
pub enum RubricaError {
    #[error("{source}")]
    Store {
        dir: PathBuf,
        #[source]
        source: StoreError,
    },
    #[error("The profile was changed since it was read")]
    Changed,
    #[error("Some fields are not valid")]
    Invalid(Vec<FieldIssue>),
}

/// The folder of the profiles and the file of the state. The store is opened at every call, so that a fix made in
/// a terminal (permissions, a file put back) takes effect without restarting the app.
pub struct Rubrica {
    dir: PathBuf,
    state: Arc<UiStateFile>,
}

impl Rubrica {
    pub fn new(dir: PathBuf, state: Arc<UiStateFile>) -> Self {
        Self { dir, state }
    }

    /// The folder of the phase 1: `$XDG_CONFIG_HOME/galleria` or `$HOME/.config/galleria`.
    pub fn default_dir() -> PathBuf {
        ProfileStore::default_dir()
    }

    fn fail(&self, source: StoreError) -> RubricaError {
        RubricaError::Store {
            dir: self.dir.clone(),
            source,
        }
    }

    /// Every profile that can be read, the order of the groups and the files that were skipped. A rubrica that does
    /// not exist yet is an empty one, and a read does not create it.
    ///
    /// # Errors
    ///
    /// When the store itself cannot be used: wrong permissions or owner, not readable, `galleria.toml` broken.
    pub fn list(&self) -> Result<RubricaView, RubricaError> {
        let exists = self.dir.try_exists().map_err(|e| self.fail(e.into()))?;
        if !exists {
            return Ok(RubricaView {
                profiles: vec![],
                groups: vec![],
                skipped: vec![],
            });
        }
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let (profiles, skipped) = store.list_lenient().map_err(|e| self.fail(e))?;
        let settings = store.settings().map_err(|e| self.fail(e))?;
        let state = self.state.load();
        Ok(RubricaView {
            profiles: profiles
                .iter()
                .map(|p| summarize(p, state.last_used.get(&p.id).map(String::as_str)))
                .collect(),
            groups: settings.groups,
            skipped: skipped.iter().map(skipped_view).collect(),
        })
    }

    /// One profile in full, without secrets.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotFound`] when there is no such profile; the other errors of [`Rubrica::list`].
    pub fn get(&self, id: Uuid) -> Result<ProfileDetail, RubricaError> {
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let profile = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        let state = self.state.load();
        Ok(detail(
            &profile,
            state.last_used.get(&profile.id).map(String::as_str),
        ))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Changes the fields named by the patch and saves the profile, atomically and with the permissions of the phase 1.
    /// `expected_modified` is the time of modification the window knows: if the file says another one, the CLI or
    /// another window wrote it in the meantime, and nothing is written. The secrets are not opened.
    ///
    /// # Errors
    ///
    /// [`RubricaError::Changed`] for a stale `expected_modified`, [`RubricaError::Invalid`] with every field that is
    /// not good (nothing is written), the errors of the store otherwise.
    pub fn update(
        &self,
        id: Uuid,
        expected_modified: &str,
        patch: ProfilePatch,
    ) -> Result<ProfileDetail, RubricaError> {
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let mut profile = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        if profile.modified != expected_modified {
            return Err(RubricaError::Changed);
        }
        let issues = apply(&mut profile, patch);
        if !issues.is_empty() {
            return Err(RubricaError::Invalid(issues));
        }
        if let Ok(now) = OffsetDateTime::now_utc().format(&Rfc3339) {
            profile.modified = now;
        }
        store.save(&profile).map_err(|e| self.fail(e))?;
        let state = self.state.load();
        Ok(detail(
            &profile,
            state.last_used.get(&profile.id).map(String::as_str),
        ))
    }
}

fn skipped_view(skipped: &Skipped) -> SkippedView {
    SkippedView {
        file_name: skipped.file_name.clone(),
        code: match skipped.error {
            StoreError::NewerSchema { .. } => "gui_profile_newer",
            _ => "gui_profile_invalid",
        },
    }
}

fn host_port(host: &str, port: Option<u16>) -> String {
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    port.map_or(host.clone(), |port| format!("{host}:{port}"))
}

/// The row of the list for a profile.
pub fn summarize(profile: &Profile, last_used: Option<&str>) -> ProfileSummary {
    let server = match (&profile.openvpn, &profile.wireguard) {
        (Some(ovpn), _) => openvpn_view(&ovpn.config, &profile.secret_refs)
            .remotes
            .first()
            .map(|r| host_port(&r.host, r.port)),
        (_, Some(wg)) => wg.peers.first().and_then(|p| p.endpoint.clone()),
        _ => None,
    };
    ProfileSummary {
        id: profile.id,
        name: profile.name.clone(),
        protocol: profile.protocol,
        group: profile.group.clone(),
        favorite: profile.favorite,
        server,
        last_used: last_used.map(str::to_owned),
        modified: profile.modified.clone(),
    }
}

fn detail(profile: &Profile, last_used: Option<&str>) -> ProfileDetail {
    ProfileDetail {
        summary: summarize(profile, last_used),
        notes: profile.notes.clone(),
        tags: profile.tags.clone(),
        created: profile.created.clone(),
        settings: profile.settings.clone(),
        secret_refs: profile.secret_refs.clone(),
        openvpn: profile
            .openvpn
            .as_ref()
            .map(|o| openvpn_view(&o.config, &profile.secret_refs)),
        wireguard: profile.wireguard.clone(),
    }
}

/// `udp` or `tcp` from the value of a `proto` directive (`tcp-client`, `udp6`…); `None` for anything else.
fn transport(value: &str) -> Option<&'static str> {
    let value = value.to_ascii_lowercase();
    if value.starts_with("tcp") {
        Some("tcp")
    } else if value.starts_with("udp") {
        Some("udp")
    } else {
        None
    }
}

/// A `remote` as read, before the defaults of `proto` and `port`: host, port, its own transport, the `<connection>`
/// block it sits in.
type RawRemote = (String, Option<u16>, Option<&'static str>, Option<usize>);

/// The remotes and the ways of authenticating of an OpenVPN configuration. A configuration that does not parse gives
/// no remotes: the text itself is still shown, and the validation is the business of the editor (T7).
pub fn openvpn_view(config: &str, secret_refs: &[String]) -> OpenVpnView {
    let items = galleria_ovpn::parse(config).unwrap_or_default();
    let mut auth = AuthFlags {
        certificate: secret_refs.iter().any(|r| r == "openvpn.block.pkcs12"),
        ..AuthFlags::default()
    };
    // The `proto` and `port` of the top level apply to every remote that says nothing; a `<connection>` has its own.
    let (mut global_proto, mut global_port) = (None, None);
    let mut block_protos: Vec<Option<&'static str>> = vec![];
    let mut block: Option<usize> = None;
    // (host, port, own transport, the block it is in)
    let mut raw: Vec<RawRemote> = vec![];
    for item in &items {
        match item {
            Item::ConnectionOpen { .. } => {
                block = Some(block_protos.len());
                block_protos.push(None);
            }
            Item::ConnectionClose { .. } => block = None,
            Item::Block { tag, .. } if tag == "cert" || tag == "pkcs12" => {
                auth.certificate = true;
            }
            Item::Directive { name, args, .. } => match name.as_str() {
                "proto" => {
                    let value = args.first().and_then(|a| transport(a));
                    match block {
                        Some(i) => block_protos[i] = value,
                        None => global_proto = value,
                    }
                }
                "port" if block.is_none() => {
                    global_port = args.first().and_then(|a| a.parse::<u16>().ok());
                }
                "remote" => {
                    if let Some(host) = args.first() {
                        raw.push((
                            host.clone(),
                            args.get(1).and_then(|a| a.parse().ok()),
                            args.get(2).and_then(|a| transport(a)),
                            block,
                        ));
                    }
                }
                "cert" | "pkcs12" => auth.certificate = true,
                "auth-user-pass" => auth.user_pass = true,
                "static-challenge" => auth.otp = true,
                _ => {}
            },
            Item::Block { .. } => {}
        }
    }
    let remotes = raw
        .into_iter()
        .map(|(host, port, own, block)| Remote {
            host,
            port: port.or(global_port),
            transport: Some(
                own.or_else(|| block.and_then(|i| block_protos[i]))
                    .or(global_proto)
                    // what OpenVPN itself does when no `proto` says otherwise
                    .unwrap_or("udp")
                    .to_owned(),
            ),
        })
        .collect();
    OpenVpnView {
        config: config.to_owned(),
        remotes,
        auth,
    }
}
