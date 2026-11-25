//! The text of a profile as a person edits it (plan B3, T7): what is wrong with an OpenVPN configuration, line by
//! line, and the `.conf` of a WireGuard profile. Only additions to the phase 1: the classification is the one the
//! import and the daemon already use (`galleria_ovpn::classify`), and no file is read.
use crate::WireGuardProfile;
use galleria_ipc::Secret;
use galleria_ovpn::{self as ovpn, Class, FILE_BLOCKS, Item, ParseError, SECRET_BLOCKS};

/// What is wrong with a line of a configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigIssueKind {
    /// The text does not parse (an open quote, a block that is not closed, a line that is too long).
    Syntax,
    /// A directive that runs a program or touches the machine (`up`, `plugin`, `auth-user-pass file`…): never run (D19).
    Dangerous,
    /// A directive or a block that Galleria does not know.
    Unsupported,
    /// A directive the daemon sets by itself (`dev`, `management`, `script-security`…).
    DaemonOwned,
    /// A directive that names a file (`ca ca.crt`): files come in with the import.
    FileReference,
    /// A block that holds a key (`<key>`, `<tls-crypt>`…): keys come in with the import.
    SecretBlock,
    /// Neither `client` nor `tls-client`: a warning, the text can be saved.
    NoTlsClient,
}

impl ConfigIssueKind {
    /// Everything but the missing `client` stops a save (Q32).
    pub fn blocks_saving(self) -> bool {
        self != Self::NoTlsClient
    }
}

/// One finding. `directive` is the *name* of the directive or the tag of the block (empty for a syntax error), never an
/// argument and never the content of a block: the finding may be shown and logged. `line` is 1-based; 0 is a problem of
/// the text as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigIssue {
    pub line: usize,
    pub directive: String,
    pub kind: ConfigIssueKind,
}

/// The classification of the phase 1 (`galleria_ovpn::classify`, as `check_strict` applies it), for the editor of the
/// raw text: every directive that would not be saved, in the order of the text, then the warning of the missing
/// `client` at line 0. A syntax error is the only finding: nothing after it can be told.
pub fn check_openvpn_config(text: &str) -> Vec<ConfigIssue> {
    let items = match ovpn::parse(text) {
        Ok(items) => items,
        Err(ParseError::Syntax { line } | ParseError::UnterminatedBlock { line }) => {
            return vec![ConfigIssue {
                line,
                directive: String::new(),
                kind: ConfigIssueKind::Syntax,
            }];
        }
    };
    let mut issues = vec![];
    for item in &items {
        let found = match item {
            Item::Directive { line, name, args } => match ovpn::classify(name, args) {
                Class::Allowed => None,
                Class::DaemonOwned => Some((*line, name, ConfigIssueKind::DaemonOwned)),
                Class::FileRef => Some((*line, name, ConfigIssueKind::FileReference)),
                Class::Dangerous => Some((*line, name, ConfigIssueKind::Dangerous)),
                Class::Unknown => Some((*line, name, ConfigIssueKind::Unsupported)),
            },
            Item::Block { line, tag, .. } => {
                if SECRET_BLOCKS.contains(&tag.as_str()) {
                    Some((*line, tag, ConfigIssueKind::SecretBlock))
                } else if FILE_BLOCKS.contains(&tag.as_str()) {
                    None
                } else {
                    Some((*line, tag, ConfigIssueKind::Unsupported))
                }
            }
            Item::ConnectionOpen { .. } | Item::ConnectionClose { .. } => None,
        };
        if let Some((line, name, kind)) = found {
            issues.push(ConfigIssue {
                line,
                directive: name.clone(),
                kind,
            });
        }
    }
    if !ovpn::has_directive(&items, "client") && !ovpn::has_directive(&items, "tls-client") {
        issues.push(ConfigIssue {
            line: 0,
            directive: "tls-client".into(),
            kind: ConfigIssueKind::NoTlsClient,
        });
    }
    issues
}

/// The keys of a WireGuard profile, kept in the archive of the secrets: the private key and the shared key of each peer,
/// in the order of the peers (`None` where a peer has none).
pub struct WireGuardKeys {
    pub private_key: Secret,
    pub preshared: Vec<Option<Secret>>,
}

fn join(items: &[String]) -> String {
    items.join(", ")
}

/// The `.conf` of a WireGuard profile. Without `keys` the lines `PrivateKey` and `PresharedKey` are not there at all:
/// that is the text the window shows. With them it is the file «Export» writes, and `import_wireguard` reads it back
/// into the same profile and the same secrets. What is not set (DNS, MTU, endpoint, keepalive) is left out.
pub fn render_wireguard(p: &WireGuardProfile, keys: Option<&WireGuardKeys>) -> String {
    let mut out = String::from("[Interface]\n");
    if let Some(keys) = keys {
        out.push_str(&format!("PrivateKey = {}\n", keys.private_key.expose()));
    }
    if !p.addresses.is_empty() {
        out.push_str(&format!("Address = {}\n", join(&p.addresses)));
    }
    // the import tells servers from search domains by looking at each item, so they go back on one line
    let dns: Vec<String> = p.dns.iter().chain(&p.search_domains).cloned().collect();
    if !dns.is_empty() {
        out.push_str(&format!("DNS = {}\n", join(&dns)));
    }
    if let Some(mtu) = p.mtu {
        out.push_str(&format!("MTU = {mtu}\n"));
    }
    for (i, peer) in p.peers.iter().enumerate() {
        out.push_str("\n[Peer]\n");
        out.push_str(&format!("PublicKey = {}\n", peer.public_key));
        if let Some(Some(psk)) = keys.and_then(|k| k.preshared.get(i)) {
            out.push_str(&format!("PresharedKey = {}\n", psk.expose()));
        }
        if let Some(endpoint) = &peer.endpoint {
            out.push_str(&format!("Endpoint = {endpoint}\n"));
        }
        if !peer.allowed_ips.is_empty() {
            out.push_str(&format!("AllowedIPs = {}\n", join(&peer.allowed_ips)));
        }
        if let Some(keepalive) = peer.persistent_keepalive {
            out.push_str(&format!("PersistentKeepalive = {keepalive}\n"));
        }
    }
    out
}
