#![allow(clippy::unwrap_used, clippy::expect_used)]
#[test]
fn harden_process_makes_it_not_dumpable() {
    galleriad::harden_process();
    assert_eq!(
        rustix::process::dumpable_behavior().unwrap(),
        rustix::process::DumpableBehavior::NotDumpable
    );
}
