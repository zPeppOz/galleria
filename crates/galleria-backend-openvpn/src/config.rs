use crate::version::*;
use galleria_ipc::*;
use galleria_ovpn::*;
use ipnet::IpNet;
use std::{collections::BTreeMap, path::Path};
use zeroize::Zeroize;
pub struct ValidOpenVpn {
    pub(crate) items: Vec<Item>,
    pub(crate) credentials: InputValues,
    pub(crate) secrets: BTreeMap<String, Secret>,
}
impl Drop for ValidOpenVpn {
    fn drop(&mut self) {
        for i in &mut self.items {
            if let Item::Block { body, .. } = i {
                body.zeroize();
            }
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Redirect {
    pub ipv4: bool,
    pub ipv6: bool,
}
pub(crate) fn redirect(args: &[String]) -> Redirect {
    Redirect {
        ipv4: !args.iter().any(|a| a == "!ipv4"),
        ipv6: args.iter().any(|a| a == "ipv6"),
    }
}
pub fn validate(p: &OpenVpnParams) -> Result<ValidOpenVpn, RpcError> {
    let items = parse(&p.config)
        .map_err(|_| ErrorCode::InvalidConfig.to_rpc("Invalid OpenVPN configuration syntax"))?;
    if let Err(rejected) = check_strict(&items) {
        let mut e =
            ErrorCode::ProfileRejected.to_rpc("OpenVPN profile contains unsupported directives");
        e.data = Some(
            serde_json::json!({"code":"profile_rejected","directives":rejected.iter().map(|r|serde_json::json!({"line":r.line,"name":r.name,"class":format!("{:?}",r.class)})).collect::<Vec<_>>() }),
        );
        return Err(e);
    }
    // OpenVPN reads physical lines into a fixed 256-byte buffer; canonical quoting
    // must not move unvalidated text past that boundary.
    if render(&items)
        .split_inclusive('\n')
        .any(|line| line.len() > 255)
    {
        return Err(ErrorCode::InvalidConfig.to_rpc("OpenVPN configuration line is too long"));
    }
    for (tag, body) in &p.secret_blocks {
        if !SECRET_BLOCKS.contains(&tag.as_str())
            || body.expose().contains(['\0', '<', '>'])
            || body
                .expose()
                .split_inclusive('\n')
                .any(|line| line.len() > 255)
        {
            return Err(ErrorCode::InvalidConfig.to_rpc("Invalid OpenVPN secret block"));
        }
    }
    Ok(ValidOpenVpn {
        credentials: InputValues {
            username: p.username.clone(),
            password: p.password.clone(),
            otp: p.otp.clone(),
            key_passphrase: p.key_passphrase.clone(),
        },
        items,
        secrets: p.secret_blocks.clone(),
    })
}
pub fn profile_redirect(p: &ValidOpenVpn) -> Redirect {
    let mut r = Redirect::default();
    for i in &p.items {
        if let Item::Directive { name, args, .. } = i
            && name == "redirect-gateway"
        {
            let n = redirect(args);
            r.ipv4 |= n.ipv4;
            r.ipv6 |= n.ipv6;
        }
    }
    r
}
pub fn profile_ipv6_routes(p: &ValidOpenVpn) -> Vec<IpNet> {
    p.items
        .iter()
        .filter_map(|i| match i {
            Item::Directive { name, args, .. } if name == "route-ipv6" => {
                args.first()?.parse().ok()
            }
            _ => None,
        })
        .collect()
}
pub struct GenInput<'a> {
    pub profile: &'a ValidOpenVpn,
    pub version: OpenVpnVersion,
    pub mgmt_socket: &'a Path,
    pub hook_socket: &'a Path,
    pub daemon_exe: &'a Path,
    pub options: &'a ConnectOptions,
}
pub fn generate_config(input: &GenInput) -> String {
    let mut s = String::new();
    // OpenVPN applies the first matching pull-filter. Daemon policy must win.
    if input.options.ignore_pushed_routes {
        s.push_str("pull-filter ignore \"route \"\npull-filter ignore \"route-ipv6 \"\npull-filter ignore \"redirect-gateway\"\n");
    }
    s.push_str(&render(&input.profile.items));
    s.push_str(&format!("management {} unix\nmanagement-client-user root\nmanagement-hold\nmanagement-query-passwords\nauth-retry none\nconnect-retry-max 1\nserver-poll-timeout 5\nverb 3\ndev {}\n",quote(&input.mgmt_socket.to_string_lossy()),galleria_backend::OVPN_INTERFACE));
    if !has_directive(&input.profile.items, "dev-type") {
        s.push_str("dev-type tun\n");
    }
    let hook = format!(
        "{} ovpn-hook {}",
        input.daemon_exe.display(),
        input.hook_socket.display()
    );
    s.push_str(&format!(
        "script-security 2\nup {}\ndown {}\n",
        quote(&hook),
        quote(&hook)
    ));
    if input.version >= (OpenVpnVersion { major: 2, minor: 7 }) {
        s.push_str("dns-updown disable\n");
    }
    for (tag, body) in &input.profile.secrets {
        s.push_str(&format!(
            "<{tag}>\n{}\n</{tag}>\n",
            body.expose().trim_end_matches('\n')
        ));
    }
    s
}

pub fn profile_pull_filters(p: &ValidOpenVpn) -> Vec<(String, String)> {
    p.items
        .iter()
        .filter_map(|item| match item {
            Item::Directive { name, args, .. } if name == "pull-filter" && args.len() == 2 => {
                Some((args[0].clone(), args[1].clone()))
            }
            _ => None,
        })
        .collect()
}
