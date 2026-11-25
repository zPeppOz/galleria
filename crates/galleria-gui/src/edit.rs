//! What the window may change in a profile (plan B3, T5): the patch it sends, the rules the new values must follow and
//! the way the patch is applied. No type of Tauri in here, and no secret: a patch has no field for one, and applying it
//! never opens the store of the secrets.
//!
//! The rules for WireGuard are the syntactic ones of `galleria-backend-wireguard/src/validate.rs` (P8). That crate is
//! not a dependency of the window (spec §3.2); a test of this crate compares the two on a table of profiles, and the
//! daemon stays the authority when the connection is made.

use base64::{Engine, engine::general_purpose::STANDARD};
use galleria_profile::{OpenVpnProfile, Profile, Settings, WireGuardProfile, check_openvpn_config};
use ipnet::IpNet;
use serde::{Deserialize, Deserializer, Serialize};
use std::net::{IpAddr, SocketAddr};

/// The longest name of a profile and of a group, in characters.
pub const NAME_MAX: usize = 64;
pub const GROUP_MAX: usize = 48;

/// One field that the window may not save, by its path in the patch (`name`, `mtu`, `peers.0.endpoint`) and the reason.
/// The webview has a text for each `code`; the path says which field gets the focus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldIssue {
    pub field: String,
    pub code: &'static str,
}

fn issue(field: impl Into<String>, code: &'static str) -> FieldIssue {
    FieldIssue {
        field: field.into(),
        code,
    }
}

/// A key that is there with the value `null` is a request («no group»); a key that is not there asks for nothing.
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

/// The fields the window can change, all optional. Unknown keys are refused, so the webview cannot reach what this
/// type does not name: not the id, not the dates, not the secrets, not a path.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilePatch {
    pub name: Option<String>,
    /// `Some(None)` is «no group».
    #[serde(default, deserialize_with = "present")]
    pub group: Option<Option<String>>,
    pub favorite: Option<bool>,
    pub notes: Option<String>,
    pub settings: Option<Settings>,
    /// Only for a WireGuard profile, and with the same number of peers: the keys in the archive of the secrets are
    /// indexed by the position of the peer.
    pub wireguard: Option<WireGuardProfile>,
    /// The text of the editor, for an OpenVPN profile only: saved as written if nothing in it stops a save (T7, Q32).
    pub openvpn_config: Option<String>,
}

/// Why a label (the name of a profile, of a group) is not good: empty, too long or with control characters.
pub fn check_label(value: &str, max: usize) -> Option<&'static str> {
    let length = value.chars().count();
    if length == 0 {
        Some("required")
    } else if length > max {
        Some("too_long")
    } else if value.chars().any(char::is_control) {
        Some("invalid_chars")
    } else {
        None
    }
}

fn endpoint(value: &str) -> bool {
    if let Ok(addr) = value.parse::<SocketAddr>() {
        return addr.port() != 0;
    }
    let Some((host, port)) = value.rsplit_once(':') else {
        return false;
    };
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && port.parse::<u16>().is_ok_and(|n| n > 0)
}

fn domain(value: &str) -> bool {
    let v = value.strip_prefix('~').unwrap_or(value);
    if v == "." {
        return true;
    }
    let v = v.strip_suffix('.').unwrap_or(v);
    v.len() <= 253
        && v.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
}

fn key(value: &str) -> bool {
    STANDARD.decode(value).is_ok_and(|bytes| bytes.len() == 32)
}

/// Everything wrong with the WireGuard settings, all at once, each with the path of its field. The daemon refuses the
/// same things (and the tests compare them); this is also stricter in two places that D26 asks for: a tunnel needs an
/// address, and a peer needs a network to route.
pub fn check_wireguard(p: &WireGuardProfile) -> Vec<FieldIssue> {
    let mut out = vec![];
    if p.addresses.is_empty() {
        out.push(issue("addresses", "required"));
    } else if p.addresses.iter().any(|a| a.parse::<IpNet>().is_err()) {
        out.push(issue("addresses", "invalid_cidr"));
    }
    if p.dns.iter().any(|d| d.parse::<IpAddr>().is_err()) {
        out.push(issue("dns", "invalid_ip"));
    }
    if !p.search_domains.iter().all(|d| domain(d)) {
        out.push(issue("search_domains", "invalid_domain"));
    }
    if p.mtu.is_some_and(|m| !(576..=9000).contains(&m)) {
        out.push(issue("mtu", "out_of_range"));
    }
    if p.peers.is_empty() {
        out.push(issue("peers", "required"));
    }
    for (i, peer) in p.peers.iter().enumerate() {
        let at = |name: &str| format!("peers.{i}.{name}");
        if !key(&peer.public_key) {
            out.push(issue(at("public_key"), "invalid_key"));
        }
        if peer.endpoint.as_deref().is_some_and(|e| !endpoint(e)) {
            out.push(issue(at("endpoint"), "invalid_endpoint"));
        }
        if peer.allowed_ips.is_empty() {
            out.push(issue(at("allowed_ips"), "required"));
        } else if peer.allowed_ips.iter().any(|n| n.parse::<IpNet>().is_err()) {
            out.push(issue(at("allowed_ips"), "invalid_cidr"));
        }
        if peer
            .persistent_keepalive
            .is_some_and(|k| k > u32::from(u16::MAX))
        {
            out.push(issue(at("persistent_keepalive"), "out_of_range"));
        }
    }
    out
}

/// Puts the patch on the profile. Nothing is changed when something is wrong: the profile is touched only if the list
/// of problems comes back empty (it is the caller that saves). `modified` is not touched here.
pub fn apply(profile: &mut Profile, patch: ProfilePatch) -> Vec<FieldIssue> {
    let mut issues = vec![];
    let mut next = profile.clone();
    if let Some(name) = patch.name {
        let name = name.trim().to_owned();
        match check_label(&name, NAME_MAX) {
            Some(code) => issues.push(issue("name", code)),
            None => next.name = name,
        }
    }
    match patch.group {
        Some(Some(group)) => match check_label(&group, GROUP_MAX) {
            Some(code) => issues.push(issue("group", code)),
            None => next.group = Some(group),
        },
        Some(None) => next.group = None,
        None => {}
    }
    if let Some(favorite) = patch.favorite {
        next.favorite = favorite;
    }
    if let Some(notes) = patch.notes {
        next.notes = notes;
    }
    if let Some(settings) = patch.settings {
        next.settings = settings;
    }
    if let Some(wireguard) = patch.wireguard {
        match &profile.wireguard {
            None => issues.push(issue("wireguard", "wrong_protocol")),
            Some(current) => {
                if wireguard.peers.len() != current.peers.len() {
                    issues.push(issue("peers", "count_changed"));
                } else {
                    let found = check_wireguard(&wireguard);
                    if found.is_empty() {
                        next.wireguard = Some(wireguard);
                    }
                    issues.extend(found);
                }
            }
        }
    }
    if let Some(text) = patch.openvpn_config {
        match &profile.openvpn {
            None => issues.push(issue("openvpn_config", "wrong_protocol")),
            // What is in the editor is what is saved, comments and all (Q32): the text is checked, never changed. The
            // check is the same one the window runs while the text is typed.
            Some(_) => {
                if check_openvpn_config(&text)
                    .iter()
                    .any(|i| i.kind.blocks_saving())
                {
                    issues.push(issue("openvpn_config", "config_issues"));
                } else {
                    next.openvpn = Some(OpenVpnProfile { config: text });
                }
            }
        }
    }
    if issues.is_empty() {
        *profile = next;
    }
    issues
}
