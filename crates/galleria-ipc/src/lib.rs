#![forbid(unsafe_code)]

pub const PROTOCOL_VERSION: u32 = 1;
pub const PROTOCOL_MIN: u32 = 1;
pub const PROTOCOL_MAX: u32 = 1;
pub const MAX_MESSAGE_BYTES: usize = 1_048_576;
pub const DEFAULT_SOCKET_PATH: &str = "/run/galleria/galleriad.sock";

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn protocol_constants_match_the_spec() {
        assert_eq!((PROTOCOL_MIN, PROTOCOL_VERSION, PROTOCOL_MAX), (1, 1, 1));
        assert_eq!(MAX_MESSAGE_BYTES, 1_048_576);
        assert_eq!(DEFAULT_SOCKET_PATH, "/run/galleria/galleriad.sock");
    }
}
