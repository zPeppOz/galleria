#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpenVpnVersion {
    pub major: u32,
    pub minor: u32,
}
pub const MIN_VERSION: OpenVpnVersion = OpenVpnVersion { major: 2, minor: 6 };
pub fn parse_version(first_line: &str) -> Option<OpenVpnVersion> {
    let s = first_line
        .strip_prefix("OpenVPN ")?
        .split_whitespace()
        .next()?;
    let mut v = s.split('.');
    Some(OpenVpnVersion {
        major: v.next()?.parse().ok()?,
        minor: v.next()?.parse().ok()?,
    })
}
