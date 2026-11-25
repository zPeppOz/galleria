//! The file «Export» writes (plan B3, T8, Q31): the profile in its native form, `.ovpn` or `.conf`, with the keys put
//! back in. Only an addition to the phase 1; it reads the archive of the secrets and writes nothing.
use crate::{Profile, SecretStore, StoreError, WireGuardKeys, render_wireguard};
use galleria_ipc::Protocol;
use zeroize::Zeroizing;

fn missing() -> StoreError {
    StoreError::Invalid("Profile is missing required key material".into())
}

/// The text of the profile as its tool wants it. OpenVPN: the configuration with every block of `secret_refs` put back
/// in line (`<key>…</key>`, `<tls-crypt>…</tls-crypt>`, in the order of `SECRET_BLOCKS`), so that `import_openvpn`
/// takes them out again into the same secrets. WireGuard: `render_wireguard` with the private key and the shared keys.
///
/// The text holds keys: it is a `Zeroizing<String>`, built in one allocation so that nothing is left behind by growing.
///
/// # Errors
///
/// A secret that the profile names and the archive does not have (nothing partial is returned), or whatever the
/// archive says.
pub fn export_config(
    profile: &Profile,
    secrets: &dyn SecretStore,
) -> Result<Zeroizing<String>, StoreError> {
    match profile.protocol {
        Protocol::Openvpn => {
            let config = &profile.openvpn.as_ref().ok_or_else(missing)?.config;
            let mut blocks = vec![];
            for tag in galleria_ovpn::SECRET_BLOCKS {
                let key = format!("openvpn.block.{tag}");
                if profile.secret_refs.contains(&key) {
                    let secret = secrets.get(&profile.id, &key)?.ok_or_else(missing)?;
                    blocks.push((*tag, secret));
                }
            }
            let extra: usize = blocks
                .iter()
                .map(|(tag, secret)| 2 * tag.len() + secret.expose().len() + 8)
                .sum();
            let mut text = Zeroizing::new(String::with_capacity(config.len() + 1 + extra));
            text.push_str(config);
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            for (tag, secret) in &blocks {
                let body = secret.expose();
                text.push('<');
                text.push_str(tag);
                text.push_str(">\n");
                text.push_str(body);
                if !body.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str("</");
                text.push_str(tag);
                text.push_str(">\n");
            }
            Ok(text)
        }
        Protocol::Wireguard => {
            let wireguard = profile.wireguard.as_ref().ok_or_else(missing)?;
            let private_key = secrets
                .get(&profile.id, "wireguard.private_key")?
                .ok_or_else(missing)?;
            let mut preshared = vec![];
            for i in 0..wireguard.peers.len() {
                let key = format!("wireguard.peer.{i}.preshared_key");
                preshared.push(if profile.secret_refs.contains(&key) {
                    Some(secrets.get(&profile.id, &key)?.ok_or_else(missing)?)
                } else {
                    None
                });
            }
            let keys = WireGuardKeys {
                private_key,
                preshared,
            };
            Ok(Zeroizing::new(render_wireguard(wireguard, Some(&keys))))
        }
    }
}
