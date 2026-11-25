//! Import from a file or from pasted text (plan B4, T1). The files are the fixtures of the GUI (`galleria-gui/tests/support`),
//! so that what is tried here is what the window, the smoke test and the screen reader check try.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../galleria-gui/tests/support/fixtures.rs"]
mod fixtures;

use fixtures::*;
use galleria_ipc::Protocol;
use galleria_profile::*;
use rstest::rstest;
use std::{
    fs,
    os::unix::fs::symlink,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

fn no_files(_: &str) -> std::io::Result<String> {
    Err(std::io::ErrorKind::NotFound.into())
}

fn sorted(secrets: &[(String, galleria_ipc::Secret)]) -> Vec<(String, String)> {
    let mut list: Vec<_> = secrets
        .iter()
        .map(|(k, v)| (k.clone(), v.expose().to_owned()))
        .collect();
    list.sort();
    list
}

#[rstest]
#[case("client\nremote x 1\n", Protocol::Openvpn)]
#[case("[Interface]\nPrivateKey = x\n", Protocol::Wireguard)]
#[case("\u{feff}[Interface]\n", Protocol::Wireguard)]
#[case("# a comment\n; another\n\n  [Interface]  \n", Protocol::Wireguard)]
#[case("[interface]\nPrivateKey = x\n", Protocol::Wireguard)]
#[case("# [Interface]\nclient\n", Protocol::Openvpn)]
#[case("client\n[Interface]\n", Protocol::Openvpn)]
#[case("", Protocol::Openvpn)]
fn detect_protocol_follows_the_first_meaningful_line(#[case] text: &str, #[case] want: Protocol) {
    assert_eq!(detect_protocol(text), want, "{text:?}");
}

/// What `import_file` makes of a folder of fixtures.
fn folder() -> (tempfile::TempDir, Fixtures) {
    let dir = tempfile::tempdir().unwrap();
    let fixtures = write_fixtures(dir.path());
    (dir, fixtures)
}

#[test]
fn import_file_reads_inline_and_separate_files_into_the_same_profile() {
    let (_dir, f) = folder();
    let inline = import_file(&f.inline, None).unwrap();
    let separate = import_file(&f.separate, None).unwrap();
    assert_eq!(inline.profile.name, "inline");
    assert_eq!(separate.profile.name, "separate");
    assert_eq!(
        inline.profile.openvpn.as_ref().unwrap().config,
        separate.profile.openvpn.as_ref().unwrap().config
    );
    assert_eq!(sorted(&inline.secrets), sorted(&separate.secrets));
    assert_eq!(
        inline
            .profile
            .secret_refs
            .iter()
            .collect::<std::collections::BTreeSet<_>>(),
        separate
            .profile
            .secret_refs
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
    assert!(
        inline
            .secrets
            .iter()
            .any(|(key, _)| key == "openvpn.block.key")
    );
    // the name can be given
    assert_eq!(
        import_file(&f.inline, Some("Ufficio"))
            .unwrap()
            .profile
            .name,
        "Ufficio"
    );
}

#[test]
fn a_conf_is_told_apart_by_its_content() {
    let (_dir, f) = folder();
    let office = import_file(&f.office, None).unwrap();
    assert_eq!(office.profile.protocol, Protocol::Wireguard);
    assert_eq!(office.profile.name, "office");
    let server = import_file(&f.server, None).unwrap();
    assert_eq!(server.profile.protocol, Protocol::Openvpn);
    assert_eq!(server.profile.name, "server");
    // `.ovpn` is OpenVPN whatever it holds: a WireGuard text under that name is never read as WireGuard (the importer of
    // the phase 1 is lenient with an OpenVPN text, the CLI does the same, and the window shows what it drops)
    let dir = tempfile::tempdir().unwrap();
    let odd = dir.path().join("odd.ovpn");
    fs::write(&odd, office_conf()).unwrap();
    let odd = import_file(&odd, None).unwrap();
    assert_eq!(odd.profile.protocol, Protocol::Openvpn);
    assert!(!odd.warnings.is_empty());
}

#[test]
fn another_extension_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "profile.txt",
        "profile",
        "profile.ovpn.bak",
        "profile.conf.d",
    ] {
        let path = dir.path().join(name);
        fs::write(&path, inline_ovpn()).unwrap();
        assert!(
            matches!(
                import_file(&path, None),
                Err(ImportFileError::UnsupportedFile)
            ),
            "{name}"
        );
    }
}

#[test]
fn a_reference_outside_the_folder_is_refused() {
    let (_dir, f) = folder();
    match import_file(&f.outside, None) {
        Err(ImportFileError::Import(ImportError::OutsideFolder(file))) => {
            assert_eq!(file, "../ca.crt");
        }
        other => panic!("{:?}", other.err()),
    }
}

#[test]
fn a_symlink_that_leaves_the_folder_is_refused() {
    let (dir, f) = folder();
    fs::remove_file(dir.path().join("ca.crt")).unwrap();
    symlink("/etc/hostname", dir.path().join("ca.crt")).unwrap();
    match import_file(&f.separate, None) {
        Err(ImportFileError::Import(ImportError::OutsideFolder(file))) => {
            assert_eq!(file, "ca.crt");
        }
        other => panic!("{:?}", other.err()),
    }
}

/// Runs `f` and says how long it took: a call that waits for a FIFO never comes back.
fn within_two_seconds<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let start = Instant::now();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    let out = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("the call did not return in two seconds");
    assert!(start.elapsed() < Duration::from_secs(2));
    out
}

fn mkfifo(path: &Path) {
    assert!(Command::new("mkfifo").arg(path).status().unwrap().success());
}

#[test]
fn a_fifo_a_device_and_a_huge_file_are_refused_without_hanging() {
    let (dir, f) = folder();
    // a FIFO with nobody writing to it
    let fifo = dir.path().join("fifo.ovpn");
    mkfifo(&fifo);
    let result = within_two_seconds(move || import_file(&fifo, None).err());
    assert!(
        matches!(result, Some(ImportFileError::NotAFile)),
        "{result:?}"
    );
    // a device that never ends, behind a link with the right extension
    let zero = dir.path().join("zero.ovpn");
    symlink("/dev/zero", &zero).unwrap();
    let result = within_two_seconds(move || import_file(&zero, None).err());
    assert!(
        matches!(result, Some(ImportFileError::NotAFile)),
        "{result:?}"
    );
    // a file of 2 MiB
    let huge = dir.path().join("huge.ovpn");
    fs::write(&huge, vec![b'#'; 2 * 1024 * 1024]).unwrap();
    let result = within_two_seconds(move || import_file(&huge, None).err());
    assert!(
        matches!(result, Some(ImportFileError::TooLarge)),
        "{result:?}"
    );
    // a profile that refers to a file of 2 MiB, and to a FIFO
    fs::write(dir.path().join("ca.crt"), vec![b'#'; 2 * 1024 * 1024]).unwrap();
    let separate = f.separate.clone();
    let result = within_two_seconds(move || import_file(&separate, None).err());
    assert!(
        matches!(result, Some(ImportFileError::TooLarge)),
        "{result:?}"
    );
    fs::remove_file(dir.path().join("ca.crt")).unwrap();
    mkfifo(&dir.path().join("ca.crt"));
    let separate = f.separate.clone();
    let result = within_two_seconds(move || import_file(&separate, None).err());
    assert!(
        matches!(
            result,
            Some(ImportFileError::Import(ImportError::MissingFile(_)))
        ),
        "{result:?}"
    );
}

#[test]
fn a_file_that_is_not_utf8_is_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("binary.ovpn");
    fs::write(&path, [0xff, 0xfe, 0x00, 0x80]).unwrap();
    assert!(matches!(
        import_file(&path, None),
        Err(ImportFileError::Unreadable(_))
    ));
}

#[test]
fn import_text_has_no_folder() {
    let text = "client\nremote vpn.example 1194\nca ca.crt\n";
    match import_text(text, "x") {
        Err(ImportError::MissingFile(file)) => assert_eq!(file, "ca.crt"),
        other => panic!("{:?}", other.err()),
    }
    // pasted text with the blocks inside is a profile like any other
    let imported = import_text(&inline_ovpn(), "x").unwrap();
    assert_eq!(imported.profile.secret_refs.len(), 2);
    let wireguard = import_text(&office_conf(), "x").unwrap();
    assert_eq!(wireguard.profile.protocol, Protocol::Wireguard);
}

#[test]
fn the_phase_1_import_functions_are_untouched() {
    let text = inline_ovpn();
    let a = import_openvpn(&text, "x", &no_files).unwrap();
    let b = import_text(&text, "x").unwrap();
    assert_eq!(
        a.profile.openvpn.as_ref().unwrap().config,
        b.profile.openvpn.as_ref().unwrap().config
    );
    assert_eq!(sorted(&a.secrets), sorted(&b.secrets));
    assert_eq!(a.warnings.len(), b.warnings.len());
    let text = office_conf();
    let a = import_wireguard(&text, "x").unwrap();
    let b = import_text(&text, "x").unwrap();
    assert_eq!(
        a.profile.wireguard.as_ref().unwrap().addresses,
        b.profile.wireguard.as_ref().unwrap().addresses
    );
    assert_eq!(sorted(&a.secrets), sorted(&b.secrets));
    assert_eq!(a.warnings.len(), b.warnings.len());
}
