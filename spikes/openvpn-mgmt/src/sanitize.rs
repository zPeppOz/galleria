//! Profile sanitization (M2): the daemon never runs what a profile asks it to run.
//! Throwaway spike code.

pub struct Removed {
    pub line: usize,
    pub directive: String,
    pub reason: &'static str,
}

pub struct Sanitized {
    pub text: String,
    pub removed: Vec<Removed>,
    /// content of the inline <key> block, if any (used by key_mode = external)
    pub inline_key: Option<String>,
}

/// Directives that run programs, load code, open control channels or write files.
const BLOCKED: &[&str] = &[
    "management", "plugin", "up", "down", "route-up", "route-pre-down", "ipchange",
    "tls-verify", "client-connect", "client-disconnect", "learn-address", "auth-user-pass-verify",
    "script-security", "dns-updown", "setenv", "setenv-safe", "config", "cd", "chroot", "daemon",
    "log", "log-append", "syslog", "writepid", "status", "user", "group", "tmp-dir", "engine",
    "providers", "tls-export-cert", "iproute", "auth-retry", "up-restart", "up-delay", "verb",
    "mute", "echo", "connect-retry-max", "server-poll-timeout",
];

/// Directives whose argument may be a file path: only the inline form is accepted.
const FILE_ARGS: &[&str] = &[
    "ca", "cert", "key", "tls-auth", "tls-crypt", "tls-crypt-v2", "pkcs12", "dh", "crl-verify",
    "secret", "extra-certs", "http-proxy-user-pass",
];

pub fn sanitize(profile: &str, strip_key: bool) -> Sanitized {
    let mut out = String::new();
    let mut removed = Vec::new();
    let mut inline_key = None;
    let mut lines = profile.lines().enumerate().peekable();
    while let Some((i, raw)) = lines.next() {
        let line = raw.trim();
        // inline blocks: <tag> ... </tag>
        if let Some(tag) = line.strip_prefix('<').and_then(|l| l.strip_suffix('>')) {
            if !tag.starts_with('/') && tag != "connection" {
                let end = format!("</{tag}>");
                let mut block = String::new();
                for (_, l) in lines.by_ref() {
                    if l.trim() == end {
                        break;
                    }
                    block.push_str(l);
                    block.push('\n');
                }
                if tag == "key" && strip_key {
                    inline_key = Some(block);
                    removed.push(Removed { line: i + 1, directive: "<key>".into(), reason: "key kept in daemon memory (external key)" });
                    continue;
                }
                if tag == "key" {
                    inline_key = Some(block.clone());
                }
                out.push_str(&format!("<{tag}>\n{block}</{tag}>\n"));
                continue;
            }
            // <connection> / </connection>: kept, the directives inside are sanitized like the others
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let mut toks = line.split_whitespace();
        let name = toks.next().unwrap().trim_start_matches("--").to_ascii_lowercase();
        let args: Vec<&str> = toks.collect();
        let reason = if BLOCKED.contains(&name.as_str()) || name.starts_with("management") {
            Some("blocked directive (M2)")
        } else if FILE_ARGS.contains(&name.as_str()) && args.first().is_some_and(|a| *a != "[inline]") {
            Some("external file reference not allowed")
        } else {
            None
        };
        if let Some(reason) = reason {
            removed.push(Removed { line: i + 1, directive: name, reason });
            continue;
        }
        // credentials and passphrase never come from files: rewritten without argument
        match name.as_str() {
            "auth-user-pass" => out.push_str("auth-user-pass\n"),
            "askpass" => out.push_str("askpass\n"),
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    Sanitized { text: out, removed, inline_key }
}
