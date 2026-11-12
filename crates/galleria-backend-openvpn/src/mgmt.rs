use base64::{Engine, engine::general_purpose::STANDARD};
use std::net::{IpAddr, SocketAddr};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticChallenge {
    pub echo: bool,
    pub concat: bool,
    pub text: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    Auth,
    PrivateKey,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MgmtEvent {
    Info(String),
    Hold,
    State {
        name: String,
        detail: String,
        local_ip: Option<IpAddr>,
        remote: Option<SocketAddr>,
    },
    ByteCount {
        rx: u64,
        tx: u64,
    },
    NeedAuth {
        static_challenge: Option<StaticChallenge>,
    },
    NeedDynamicChallenge,
    NeedKeyPassphrase,
    VerificationFailed(AuthKind),
    Log {
        flags: String,
        text: String,
    },
    Fatal(String),
    Success(String),
    Error(String),
    Other(String),
}
pub fn parse_line(line: &str) -> MgmtEvent {
    let line = line.trim_end_matches(['\r', '\n']);
    if let Some(s) = line.strip_prefix(">STATE:") {
        let p: Vec<_> = s.split(',').collect();
        if p.len() >= 4 {
            return MgmtEvent::State {
                name: p[1].into(),
                detail: p[2].into(),
                local_ip: p[3].parse().ok(),
                remote: p
                    .get(4)
                    .and_then(|s| s.parse::<IpAddr>().ok())
                    .zip(p.get(5).and_then(|s| s.parse::<u16>().ok()))
                    .map(|(ip, port)| SocketAddr::new(ip, port)),
            };
        }
    }
    if let Some(s) = line.strip_prefix(">PASSWORD:") {
        if s.contains("CRV1:") {
            return MgmtEvent::NeedDynamicChallenge;
        }
        if s.starts_with("Verification Failed:") {
            return MgmtEvent::VerificationFailed(if s.contains("Private Key") {
                AuthKind::PrivateKey
            } else {
                AuthKind::Auth
            });
        }
        if s.starts_with("Need 'Private Key'") {
            return MgmtEvent::NeedKeyPassphrase;
        }
        if s.starts_with("Need 'Auth'") {
            return MgmtEvent::NeedAuth {
                static_challenge: s
                    .split_once("SC:")
                    .and_then(|(_, sc)| sc.split_once(','))
                    .and_then(|(flags, text)| {
                        flags.parse::<u32>().ok().map(|f| StaticChallenge {
                            echo: f & 1 != 0,
                            concat: f & 2 != 0,
                            text: text.into(),
                        })
                    }),
            };
        }
    }
    if let Some(s) = line.strip_prefix(">BYTECOUNT:")
        && let Some((a, b)) = s.split_once(',')
        && let (Ok(rx), Ok(tx)) = (a.parse(), b.parse())
    {
        return MgmtEvent::ByteCount { rx, tx };
    }
    if line.starts_with(">HOLD:") {
        return MgmtEvent::Hold;
    }
    if let Some(s) = line.strip_prefix(">INFO:") {
        return MgmtEvent::Info(s.into());
    }
    if let Some(s) = line.strip_prefix(">LOG:") {
        let mut p = s.splitn(3, ',');
        let _ = p.next();
        if let (Some(flags), Some(text)) = (p.next(), p.next()) {
            return MgmtEvent::Log {
                flags: flags.into(),
                text: text.into(),
            };
        }
    }
    for (prefix, kind) in [(">FATAL:", 0), ("SUCCESS:", 1), ("ERROR:", 2)] {
        if let Some(s) = line.strip_prefix(prefix) {
            return match kind {
                0 => MgmtEvent::Fatal(s.trim_start().into()),
                1 => MgmtEvent::Success(s.trim_start().into()),
                _ => MgmtEvent::Error(s.trim_start().into()),
            };
        }
    }
    MgmtEvent::Other(line.into())
}
pub fn mgmt_quote(s: &str) -> String {
    galleria_ovpn::quote(&s.replace(['\r', '\n', '\0'], ""))
}
pub fn auth_response(password: &str, otp: Option<&str>, sc: Option<&StaticChallenge>) -> String {
    match (otp, sc) {
        (Some(otp), Some(sc)) if sc.concat => format!("{password}{otp}"),
        (Some(otp), Some(_)) => format!(
            "SCRV1:{}:{}",
            STANDARD.encode(password),
            STANDARD.encode(otp)
        ),
        _ => password.into(),
    }
}
