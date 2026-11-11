#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_backend::net::*;
use rstest::rstest;
#[rstest]
#[case(vec!["0.0.0.0/0"], true)]
#[case(vec!["0.0.0.0/1", "128.0.0.0/1"], true)]
#[case(vec!["10.0.0.0/8", "192.0.2.0/24"], false)]
fn full_tunnel_detects_default_and_two_halves(#[case] routes: Vec<&str>, #[case] expected: bool) {
    assert_eq!(
        is_full_tunnel_v4(
            &routes
                .iter()
                .map(|s| s.parse().unwrap())
                .collect::<Vec<_>>()
        ),
        expected
    );
}
#[rstest]
#[case(vec!["::/0"], true)]
#[case(vec!["::/1", "8000::/1"], true)]
#[case(vec!["2000::/3"], true)]
#[case(vec!["2000::/4", "3000::/4"], true)]
#[case(vec!["2000::/4"], false)]
#[case(vec!["fd00::/8"], false)]
#[case(vec!["2001:db8::/32"], false)]
fn ipv6_default_detects_every_covering_form(#[case] routes: Vec<&str>, #[case] expected: bool) {
    assert_eq!(
        carries_ipv6_default(
            &routes
                .iter()
                .map(|s| s.parse().unwrap())
                .collect::<Vec<_>>()
        ),
        expected
    );
}
#[test]
fn split_routes_are_not_full_tunnel() {
    assert!(!is_full_tunnel_v4(&["10.0.0.0/8".parse().unwrap()]));
}
#[test]
fn ipv6_default_is_false_for_partial_cover() {
    assert!(!carries_ipv6_default(&["2000::/4".parse().unwrap()]));
}
