#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

/// Runs `galleria import` with a private config folder; returns success and the profile count.
fn import(ovpn: &Path, config: &Path) -> (bool, usize) {
    let output = Command::new(env!("CARGO_BIN_EXE_galleria"))
        .args(["import", ovpn.to_str().unwrap()])
        .env("XDG_CONFIG_HOME", config)
        .output()
        .unwrap();
    let profiles = fs::read_dir(config.join("galleria/profiles"))
        .map(|d| d.count())
        .unwrap_or(0);
    (output.status.success(), profiles)
}

fn setup(reference: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let folder = root.path().join("vpn");
    fs::create_dir_all(folder.join("certs")).unwrap();
    fs::write(folder.join("ca.crt"), "CERTIFICATE\n").unwrap();
    fs::write(root.path().join("private.key"), "OUTSIDE-SECRET\n").unwrap();
    let ovpn = folder.join("office.ovpn");
    fs::write(
        &ovpn,
        format!("client\nremote vpn.example.test 1194\nca \"{reference}\"\n"),
    )
    .unwrap();
    (root, ovpn)
}

#[test]
fn import_reads_a_file_next_to_the_ovpn() {
    let (root, ovpn) = setup("ca.crt");
    assert_eq!(import(&ovpn, &root.path().join("config")), (true, 1));
}

#[test]
fn import_refuses_absolute_and_parent_references() {
    for reference in ["../private.key", "certs/../../private.key"] {
        let (root, ovpn) = setup(reference);
        assert_eq!(
            import(&ovpn, &root.path().join("config")),
            (false, 0),
            "{reference}"
        );
    }
    let (root, ovpn) = setup("");
    let absolute = root.path().join("private.key");
    fs::write(
        &ovpn,
        format!(
            "client\nremote vpn.example.test 1194\nca \"{}\"\n",
            absolute.display()
        ),
    )
    .unwrap();
    assert_eq!(import(&ovpn, &root.path().join("config")), (false, 0));
}

#[test]
fn import_refuses_a_symlink_that_leaves_the_folder() {
    let (root, ovpn) = setup("link.crt");
    std::os::unix::fs::symlink(
        root.path().join("private.key"),
        ovpn.parent().unwrap().join("link.crt"),
    )
    .unwrap();
    assert_eq!(import(&ovpn, &root.path().join("config")), (false, 0));
}
