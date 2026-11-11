use galleria_ipc::{BackendParams, InputValues, Secret};
use std::{borrow::Cow, cell::Cell};

pub const REDACTED: &str = "[redatto]";
#[derive(Default)]
pub struct Redactor {
    secrets: Vec<Secret>,
    in_pem: Cell<bool>,
}
impl Redactor {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_secret(&mut self, value: &str) {
        for line in value.lines().filter(|line| !line.is_empty()) {
            if !self.secrets.iter().any(|s| s.expose() == line) {
                self.secrets.push(Secret::new(line));
            }
        }
        self.secrets
            .sort_by_key(|s| std::cmp::Reverse(s.expose().len()));
    }
    pub fn clear(&mut self) {
        self.secrets.clear();
        self.in_pem.set(false);
    }
    pub(crate) fn add_backend(&mut self, params: &BackendParams) {
        match params {
            BackendParams::Wireguard(p) => {
                self.add_secret(p.private_key.expose());
                for s in p.peers.iter().filter_map(|p| p.preshared_key.as_ref()) {
                    self.add_secret(s.expose());
                }
            }
            BackendParams::Openvpn(p) => {
                for s in p.secret_blocks.values().chain(
                    [
                        p.password.as_ref(),
                        p.otp.as_ref(),
                        p.key_passphrase.as_ref(),
                    ]
                    .into_iter()
                    .flatten(),
                ) {
                    self.add_secret(s.expose());
                }
                if let Some(username) = &p.username {
                    self.add_secret(username);
                }
            }
        }
    }
    pub(crate) fn add_input(&mut self, values: &InputValues) {
        for s in [&values.password, &values.otp, &values.key_passphrase]
            .into_iter()
            .flatten()
        {
            self.add_secret(s.expose());
        }
        if let Some(username) = &values.username {
            self.add_secret(username);
        }
    }
    pub fn redact<'a>(&self, line: &'a str) -> Cow<'a, str> {
        let mut output = String::new();
        for part in line.split_inclusive('\n') {
            let text = part.trim_end_matches(['\r', '\n']);
            for span in self.structural(text) {
                match span {
                    Span::Redacted => output.push_str(REDACTED),
                    Span::Original(mut rest) => {
                        // Only original input is searched; generated markers never re-enter the filter.
                        while !rest.is_empty() {
                            if let Some(secret) =
                                self.secrets.iter().find(|s| rest.starts_with(s.expose()))
                            {
                                output.push_str(REDACTED);
                                rest = &rest[secret.expose().len()..];
                            } else if let Some(ch) = rest.chars().next() {
                                output.push(ch);
                                rest = &rest[ch.len_utf8()..];
                            }
                        }
                    }
                }
            }
            output.push_str(&part[text.len()..]);
        }
        if output == line {
            Cow::Borrowed(line)
        } else {
            Cow::Owned(output)
        }
    }
    fn structural<'a>(&self, mut text: &'a str) -> Vec<Span<'a>> {
        let mut spans = Vec::new();
        loop {
            if self.in_pem.get() {
                let Some(end) = text.find("-----END ") else {
                    spans.push(Span::Redacted);
                    break;
                };
                if end > 0 {
                    spans.push(Span::Redacted);
                }
                let Some(close) = text[end + 9..].find("-----") else {
                    spans.push(Span::Redacted);
                    break;
                };
                let after = end + 9 + close + 5;
                spans.push(Span::Original(&text[end..after]));
                text = &text[after..];
                self.in_pem.set(false);
            } else {
                let Some(begin) = text.find("-----BEGIN ") else {
                    public_span(&mut spans, text);
                    break;
                };
                public_span(&mut spans, &text[..begin]);
                let Some(close) = text[begin + 11..].find("-----") else {
                    spans.push(Span::Redacted);
                    break;
                };
                let after = begin + 11 + close + 5;
                spans.push(Span::Original(&text[begin..after]));
                text = &text[after..];
                self.in_pem.set(true);
            }
            if text.is_empty() {
                break;
            }
        }
        spans
    }
}
enum Span<'a> {
    Original(&'a str),
    Redacted,
}
fn public_span<'a>(spans: &mut Vec<Span<'a>>, text: &'a str) {
    let credential = ["password \"", "username \""]
        .into_iter()
        .filter_map(|prefix| text.find(prefix).map(|start| (start, prefix.len())))
        .min_by_key(|(start, _)| *start);
    if let Some((start, len)) = credential
        && let Some(end) = text[start + len..].find('"')
    {
        spans.push(Span::Original(&text[..start + len + end + 1]));
        spans.push(Span::Original(" "));
        spans.push(Span::Redacted);
    } else {
        spans.push(Span::Original(text));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn every_backend_credential_and_input_value_is_registered() {
        let mut r = Redactor::new();
        let backend: BackendParams = serde_json::from_value(serde_json::json!({"protocol":"openvpn","config":"client","secret_blocks":{"key":"block-one\nblock-two","tls-auth":"tls-block"},"username":"test-user","password":"test-password","otp":"test-otp","key_passphrase":"test-phrase"})).unwrap();
        r.add_backend(&backend);
        for secret in [
            "block-one",
            "block-two",
            "tls-block",
            "test-user",
            "test-password",
            "test-otp",
            "test-phrase",
        ] {
            assert_eq!(r.redact(secret), REDACTED);
        }
        let backend: BackendParams = serde_json::from_value(serde_json::json!({"protocol":"wireguard","private_key":"private-value","addresses":[],"dns":[],"search_domains":[],"mtu":null,"peers":[{"public_key":"public-value","preshared_key":"preshared-value","endpoint":null,"allowed_ips":[],"persistent_keepalive":null}]})).unwrap();
        r.add_backend(&backend);
        assert_eq!(
            r.redact("private-value preshared-value public-value"),
            "[redatto] [redatto] public-value"
        );
        let input: InputValues = serde_json::from_value(serde_json::json!({"username":"new-user","password":"new-password","otp":"new-otp","key_passphrase":"new-phrase"})).unwrap();
        r.add_input(&input);
        assert_eq!(
            r.redact("new-user new-password new-otp new-phrase"),
            "[redatto] [redatto] [redatto] [redatto]"
        );
        r.clear();
        assert_eq!(r.redact("private-value"), "private-value");
    }
}
