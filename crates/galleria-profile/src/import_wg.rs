use crate::*;
use galleria_ipc::Secret;
fn list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
pub fn import_wireguard(text: &str, name: &str) -> Result<Imported, ImportError> {
    let mut wg = WireGuardProfile {
        addresses: vec![],
        dns: vec![],
        search_domains: vec![],
        mtu: None,
        peers: vec![],
    };
    let mut secrets = vec![];
    let mut warnings = vec![];
    let mut section = "";
    for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let n = index + 1;
        if line.is_empty() {
            continue;
        }
        if line == "[Interface]" {
            section = "interface";
            continue;
        }
        if line == "[Peer]" {
            section = "peer";
            wg.peers.push(WireGuardPeer::default());
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(ImportError::Syntax(n))?;
        let key = key.trim();
        let value = value.trim();
        match (section, key) {
            ("interface", "PrivateKey") => {
                secrets.push(("wireguard.private_key".into(), Secret::new(value)))
            }
            ("interface", "Address") => wg.addresses.extend(list(value)),
            ("interface", "DNS") => {
                for item in list(value) {
                    if item.parse::<std::net::IpAddr>().is_ok() {
                        wg.dns.push(item);
                    } else {
                        wg.search_domains.push(item);
                    }
                }
            }
            ("interface", "MTU") => {
                wg.mtu = Some(value.parse().map_err(|_| ImportError::Syntax(n))?)
            }
            ("peer", _) => {
                let i = wg
                    .peers
                    .len()
                    .checked_sub(1)
                    .ok_or(ImportError::Syntax(n))?;
                let peer = &mut wg.peers[i];
                match key {
                    "PublicKey" => peer.public_key = value.into(),
                    "Endpoint" => peer.endpoint = Some(value.into()),
                    "AllowedIPs" => peer.allowed_ips.extend(list(value)),
                    "PersistentKeepalive" => {
                        peer.persistent_keepalive =
                            Some(value.parse().map_err(|_| ImportError::Syntax(n))?)
                    }
                    "PresharedKey" => secrets.push((
                        format!("wireguard.peer.{i}.preshared_key"),
                        Secret::new(value),
                    )),
                    _ => warnings.push(ImportWarning {
                        line: n,
                        directive: key.into(),
                        kind: WarningKind::Unsupported,
                    }),
                }
            }
            ("interface", _) => warnings.push(ImportWarning {
                line: n,
                directive: key.into(),
                kind: if ["PreUp", "PostUp", "PreDown", "PostDown"].contains(&key) {
                    WarningKind::Dangerous
                } else {
                    WarningKind::Unsupported
                },
            }),
            _ => return Err(ImportError::Syntax(n)),
        }
    }
    if !secrets
        .iter()
        .any(|(k, v)| k == "wireguard.private_key" && !v.expose().is_empty())
        || wg.addresses.is_empty()
        || wg.peers.is_empty()
        || wg
            .peers
            .iter()
            .any(|p| p.public_key.is_empty() || p.allowed_ips.is_empty())
    {
        return Err(ImportError::Invalid);
    }
    let mut profile = Profile::new(name, ProtocolConfig::Wireguard(wg));
    profile.secret_refs = secrets.iter().map(|(k, _)| k.clone()).collect();
    Ok(Imported {
        profile,
        secrets,
        warnings,
    })
}
