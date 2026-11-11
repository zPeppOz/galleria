#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::{LogLevel, LogSource};
use galleriad::{
    logbuf::LogBuffer,
    redact::{REDACTED, Redactor},
};
use rstest::rstest;
#[test]
fn redacts_registered_secret_anywhere() {
    let mut r = Redactor::new();
    r.add_secret("alice-secret");
    assert_eq!(r.redact("pw=alice-secret ok"), "pw=[redatto] ok");
}
#[test]
fn redacts_every_line_of_a_multiline_secret() {
    let mut r = Redactor::new();
    r.add_secret("pem-line-one\r\npem-line-two\n\n");
    assert_eq!(
        r.redact("prefix pem-line-one and pem-line-two"),
        "prefix [redatto] and [redatto]"
    );
}
#[rstest]
#[case("7")]
#[case("ab")]
#[case("abc")]
#[case("abcd")]
#[case("1234567890123456789012345678901234567890123456789012345678901234")]
fn redacts_secrets_of_any_length(#[case] secret: &str) {
    let mut r = Redactor::new();
    r.add_secret(secret);
    assert_eq!(r.redact(&format!("pw={secret}")), "pw=[redatto]");
    if secret == "ab" {
        assert_eq!(r.redact("cabina"), "c[redatto]ina");
    }
}
#[test]
fn empty_secret_is_ignored() {
    let mut r = Redactor::new();
    r.add_secret("");
    r.add_secret("\n\r\n");
    assert_eq!(r.redact("ordinary line"), "ordinary line");
}
#[test]
fn longer_secrets_are_replaced_first() {
    let mut r = Redactor::new();
    r.add_secret("ab");
    r.add_secret("abcd");
    r.add_secret("a");
    assert_eq!(r.redact("x=abcd;ab"), "x=[redatto];[redatto]");
}
#[test]
fn redacts_pem_bodies_structurally() {
    let r = Redactor::new();
    assert_eq!(
        r.redact("-----BEGIN RSA PRIVATE KEY-----"),
        "-----BEGIN RSA PRIVATE KEY-----"
    );
    assert_eq!(r.redact("not-registered-base64"), REDACTED);
    assert_eq!(
        r.redact("-----END RSA PRIVATE KEY-----"),
        "-----END RSA PRIVATE KEY-----"
    );
    assert_eq!(r.redact("public again"), "public again");
    let multiline = r.redact("-----BEGIN PRIVATE KEY-----\nhidden\n-----END PRIVATE KEY-----");
    assert!(!multiline.contains("hidden"));
}
#[test]
fn redacts_management_credentials() {
    let r = Redactor::new();
    assert_eq!(
        r.redact("password \"Auth\" \"x1y2z3\""),
        "password \"Auth\" [redatto]"
    );
    assert_eq!(
        r.redact("SENT: username \"Auth\" \"alice\""),
        "SENT: username \"Auth\" [redatto]"
    );
}
fn push(b: &mut LogBuffer, n: usize) {
    b.push(LogLevel::Info, LogSource::Daemon, format!("line {n}"));
}
#[test]
fn buffer_drops_oldest_beyond_capacity() {
    let mut b = LogBuffer::with_capacity(3);
    for n in 0..5 {
        push(&mut b, n);
    }
    let (entries, last) = b.after(None, 100);
    assert_eq!(entries.iter().map(|e| e.seq).collect::<Vec<_>>(), [3, 4, 5]);
    assert_eq!(last, 5);
}
#[test]
fn after_returns_only_newer_entries() {
    let mut b = LogBuffer::with_capacity(5);
    for n in 0..5 {
        push(&mut b, n);
    }
    let (entries, last) = b.after(Some(3), 10);
    assert_eq!(entries.iter().map(|e| e.seq).collect::<Vec<_>>(), [4, 5]);
    assert_eq!(last, 5);
    assert!(b.after(Some(5), 10).0.is_empty());
}
#[test]
fn logs_limit_is_capped_at_1000() {
    let mut b = LogBuffer::with_capacity(2000);
    for n in 0..2000 {
        push(&mut b, n);
    }
    assert_eq!(b.after(None, 5000).0.len(), 1000);
}
#[test]
fn clear_preserves_sequence_for_existing_subscribers() {
    let mut b = LogBuffer::with_capacity(5);
    push(&mut b, 1);
    b.clear();
    assert!(b.after(None, 10).0.is_empty());
    push(&mut b, 2);
    assert_eq!(b.after(Some(1), 10).0[0].seq, 2);
}

#[test]
fn structural_redaction_is_not_reprocessed_as_a_short_secret() {
    let mut r = Redactor::new();
    r.add_secret("[");
    r.add_secret("a");
    assert_eq!(
        r.redact("password \"Auth\" \"unregistered\""),
        "p[redatto]ssword \"Auth\" [redatto]"
    );
}
#[test]
fn pagination_cursor_does_not_skip_unreturned_entries() {
    let mut b = LogBuffer::with_capacity(5);
    for n in 0..5 {
        push(&mut b, n);
    }
    let (first, cursor) = b.after(None, 2);
    assert_eq!(first.len(), 2);
    assert_eq!(cursor, 2);
    assert_eq!(b.after(Some(cursor), 2).0[0].seq, 3);
}

#[test]
fn original_marker_prefix_never_bypasses_secret_filtering() {
    let mut r = Redactor::new();
    r.add_secret("[redatto]abc");
    assert_eq!(
        r.redact("[redatto]abc password \"Auth\" \"other\""),
        "[redatto] password \"Auth\" [redatto]"
    );
}
#[test]
fn pem_payload_before_an_end_delimiter_is_redacted() {
    let r = Redactor::new();
    r.redact("-----BEGIN PRIVATE KEY-----");
    assert_eq!(
        r.redact("unregistered-body-----END PRIVATE KEY-----"),
        "[redatto]-----END PRIVATE KEY-----"
    );
    assert_eq!(r.redact("public again"), "public again");
}
