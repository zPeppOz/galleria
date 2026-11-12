#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Allowed,
    DaemonOwned,
    FileRef,
    Dangerous,
    Unknown,
}
pub const SECRET_BLOCKS: &[&str] = &["key", "tls-auth", "tls-crypt", "tls-crypt-v2", "pkcs12"];
pub const ALLOWED: &[&str] = &[
    "allow-pull-fqdn",
    "allow-recursive-routing",
    "client-nat",
    "proto-force",
    "local",
    "link-mtu",
    "mtu-test",
    "passtos",
    "socket-flags",
    "tcp-nodelay",
    "max-packet-size",
    "session-timeout",
    "client",
    "tls-client",
    "pull",
    "dev-type",
    "proto",
    "remote",
    "remote-random",
    "remote-random-hostname",
    "resolv-retry",
    "nobind",
    "bind",
    "lport",
    "rport",
    "port",
    "float",
    "cipher",
    "data-ciphers",
    "data-ciphers-fallback",
    "ncp-ciphers",
    "auth",
    "auth-user-pass",
    "auth-nocache",
    "static-challenge",
    "remote-cert-tls",
    "verify-x509-name",
    "tls-version-min",
    "tls-version-max",
    "tls-cipher",
    "tls-ciphersuites",
    "tls-groups",
    "tls-cert-profile",
    "key-direction",
    "comp-lzo",
    "compress",
    "allow-compression",
    "tun-mtu",
    "mssfix",
    "fragment",
    "mtu-disc",
    "sndbuf",
    "rcvbuf",
    "keepalive",
    "ping",
    "ping-restart",
    "ping-exit",
    "ping-timer-rem",
    "inactive",
    "reneg-sec",
    "reneg-bytes",
    "reneg-pkts",
    "hand-window",
    "tran-window",
    "explicit-exit-notify",
    "redirect-gateway",
    "redirect-private",
    "route",
    "route-ipv6",
    "route-nopull",
    "route-metric",
    "route-gateway",
    "pull-filter",
    "dhcp-option",
    "dns",
    "topology",
    "block-ipv6",
    "mute-replay-warnings",
    "replay-window",
    "http-proxy-option",
];
pub const OWNED: &[&str] = &[
    "dev",
    "verb",
    "mute",
    "auth-retry",
    "connect-retry",
    "connect-retry-max",
    "connect-timeout",
    "server-poll-timeout",
    "persist-tun",
    "persist-key",
    "script-security",
    "block-outside-dns",
    "ignore-unknown-option",
    "management",
    "management-hold",
    "management-client-user",
];
pub const DANGEROUS: &[&str] = &[
    "up",
    "down",
    "route-up",
    "route-pre-down",
    "ipchange",
    "tls-verify",
    "client-connect",
    "client-disconnect",
    "learn-address",
    "auth-user-pass-verify",
    "plugin",
    "setenv",
    "setenv-safe",
    "config",
    "cd",
    "chroot",
    "user",
    "group",
    "log",
    "log-append",
    "syslog",
    "status",
    "writepid",
    "daemon",
    "engine",
    "providers",
    "dns-updown",
    "iproute",
    "tmp-dir",
    "tls-export-cert",
    "up-restart",
    "up-delay",
    "echo",
    "http-proxy-user-pass",
];
pub const FILE_BLOCKS: &[&str] = &[
    "ca",
    "cert",
    "key",
    "tls-auth",
    "tls-crypt",
    "tls-crypt-v2",
    "pkcs12",
    "extra-certs",
    "crl-verify",
];
pub fn classify(name: &str, args: &[String]) -> Class {
    if name.starts_with("management") || OWNED.contains(&name) {
        return Class::DaemonOwned;
    }
    if FILE_BLOCKS.contains(&name) {
        return if args == ["[inline]"] {
            Class::Allowed
        } else {
            Class::FileRef
        };
    }
    if name == "auth-user-pass" && !args.is_empty() || name == "askpass" && !args.is_empty() {
        return Class::Dangerous;
    }
    if name == "http-proxy" {
        return if args.len() == 2
            || args.len() == 3 && ["auto", "auto-nct"].contains(&args[2].as_str())
        {
            Class::Allowed
        } else {
            Class::Dangerous
        };
    }
    if name == "socks-proxy" {
        return if (1..=2).contains(&args.len()) {
            Class::Allowed
        } else {
            Class::Dangerous
        };
    }
    if DANGEROUS.contains(&name) {
        Class::Dangerous
    } else if ALLOWED.contains(&name) {
        Class::Allowed
    } else {
        Class::Unknown
    }
}
