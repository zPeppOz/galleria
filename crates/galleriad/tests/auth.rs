use galleriad::auth::*;
use rstest::rstest;
#[rstest]
#[case::root(0,0,vec![],Some(1500),true)]
#[case::primary_group(1000,1500,vec![],Some(1500),true)]
#[case::supplementary_group(1000,1000,vec![27,1500],Some(1500),true)]
#[case::outsider(1001,1001,vec![27],Some(1500),false)]
#[case::no_group_configured(1000,1500,vec![1500],None,false)]
fn authorize_follows_group_membership(
    #[case] uid: u32,
    #[case] gid: u32,
    #[case] groups: Vec<u32>,
    #[case] allowed: Option<u32>,
    #[case] expected: bool,
) {
    assert_eq!(
        authorize(
            &PeerCreds {
                uid,
                gid,
                groups,
                pid: None
            },
            &AccessPolicy {
                allowed_gid: allowed
            }
        ),
        expected
    );
}
