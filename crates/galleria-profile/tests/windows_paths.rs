#![cfg(windows)]
//! T5: the profiles under `%APPDATA%`, the folder of the service under `%PROGRAMDATA%` is in `galleriad`, and the files
//! are for the user alone. Windows only: on Linux the same is said by `profiles.rs` and `import_file.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use galleria_ipc::Secret;
use galleria_profile::{
    FileSecretStore, ImportFileError, Profile, ProfileStore, SecretStore, import_file,
    windows_paths::profiles_dir,
};
use std::{ffi::OsString, fs, path::Path, process::Command};

const EXAMPLE: &str = r#"
schema_version = 1
id = "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11"
name = "Ufficio Napoli"
protocol = "openvpn"
group = "Lavoro"
tags = []
favorite = false
notes = ""
created = "2026-11-10T10:00:00Z"
modified = "2026-11-10T10:00:00Z"
secret_refs = []
[settings]
ipv6_block = true
auto_reconnect = true
ignore_pushed_routes = false
[openvpn]
config = "client\nremote vpn.esempio-ufficio.it 1194\n"
"#;

fn profile() -> Profile {
    Profile::from_toml(EXAMPLE).unwrap()
}

#[test]
fn profiles_live_under_appdata() {
    let appdata = OsString::from(r"C:\Users\x\AppData\Roaming");
    assert_eq!(
        profiles_dir(Some(appdata)).unwrap(),
        Path::new(r"C:\Users\x\AppData\Roaming\galleria")
    );
}

#[test]
fn an_empty_or_relative_folder_is_refused() {
    // one test and not an `rstest`: the gate `WP2` looks for this name, and a case is named `name::case_1`
    for (what, value) in [
        ("missing", None),
        ("empty", Some("")),
        ("relative name", Some("profili")),
        ("relative with a dot", Some(r".\x")),
        ("relative to the drive", Some("C:x")),
        ("root without a drive", Some(r"\x")),
    ] {
        assert_eq!(profiles_dir(value.map(OsString::from)), None, "{what}");
    }
}

/// The ACL of `path` as SDDL, through `icacls /save` (the names that `icacls` prints depend on the language of the system,
/// the SDDL does not: `WD` is Everyone, `BU` is `BUILTIN\Users`, `AU` is Authenticated Users).
fn sddl(path: &Path) -> String {
    let out = path.with_extension("acl");
    let status = Command::new("icacls")
        .arg(path)
        .arg("/save")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "icacls: {}",
        String::from_utf8_lossy(&status.stdout)
    );
    let bytes = fs::read(&out).unwrap();
    fs::remove_file(&out).unwrap();
    // `icacls /save` writes UTF-16LE; the sources differ on the BOM, so both are read (no BOM: the second byte of the
    // first ASCII character is 0). UTF-8 only when it is neither.
    let bom = bytes.starts_with(&[0xff, 0xfe]);
    if bom || bytes.get(1) == Some(&0) {
        let units: Vec<u16> = bytes[if bom { 2 } else { 0 }..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

#[test]
fn a_saved_profile_is_not_readable_by_other_users() {
    let dir = tempfile::tempdir().unwrap();
    let store = ProfileStore::open(&dir.path().join("galleria")).unwrap();
    let p = profile();
    store.save(&p).unwrap();
    // the secrets of Windows are in `FileSecretStore` (plain file until P5): the file that matters for M1
    FileSecretStore::open(&dir.path().join("galleria"))
        .unwrap()
        .set(&p.id, "password", &Secret::new("hunter2"))
        .unwrap();
    for (what, file) in [
        ("profile", format!("galleria/profiles/{}.toml", p.id)),
        ("secret", format!("galleria/secrets/{}.toml", p.id)),
    ] {
        let acl = sddl(&dir.path().join(file));
        assert!(acl.contains("D:"), "no DACL on the {what} in {acl:?}");
        assert!(
            acl.contains("(A;"),
            "a DACL with no entry is not the case to check ({what}): {acl:?}"
        );
        for (sid, name) in [
            ("S-1-1-0", "Everyone"),
            ("S-1-5-32-545", "BUILTIN\\Users"),
            ("S-1-5-11", "Authenticated Users"),
        ] {
            let alias = match sid {
                "S-1-1-0" => "WD",
                "S-1-5-32-545" => "BU",
                _ => "AU",
            };
            for trustee in [sid, alias] {
                assert!(
                    !acl.contains(&format!(";;;{trustee})")),
                    "{name} has an entry on the {what}: {acl:?}"
                );
            }
        }
    }
}

#[test]
fn a_saved_profile_reads_back_and_replaces_the_old_one() {
    let dir = tempfile::tempdir().unwrap();
    let store = ProfileStore::open(&dir.path().join("galleria")).unwrap();
    let first = profile();
    store.save(&first).unwrap();
    assert_eq!(
        store.find(&first.id.to_string()).unwrap().name,
        "Ufficio Napoli"
    );
    let second = Profile::from_toml(&EXAMPLE.replace("Ufficio Napoli", "Ufficio Roma")).unwrap();
    store.save(&second).unwrap();
    assert_eq!(
        store.find(&first.id.to_string()).unwrap().name,
        "Ufficio Roma"
    );
    let files: Vec<_> = fs::read_dir(dir.path().join("galleria/profiles"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(files, [OsString::from(format!("{}.toml", first.id))]);
    assert_eq!(store.list().unwrap().len(), 1);
}

#[test]
fn a_link_is_not_imported() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.ovpn");
    fs::write(&real, "client\nremote vpn.esempio-ufficio.it 1194\n").unwrap();
    let link = dir.path().join("link.ovpn");
    // The test needs the right to make links: if it lacks it, it fails, it does not skip (the runner is an administrator).
    std::os::windows::fs::symlink_file(&real, &link)
        .expect("the test needs the right to create symbolic links");
    assert!(import_file(&real, None).is_ok());
    let result = import_file(&link, None);
    assert!(
        matches!(result, Err(ImportFileError::NotAFile)),
        "{:?}",
        result.err()
    );
}

#[test]
fn a_folder_is_not_imported_as_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("folder.ovpn");
    fs::create_dir(&folder).unwrap();
    let result = import_file(&folder, None);
    assert!(
        matches!(result, Err(ImportFileError::NotAFile)),
        "{:?}",
        result.err()
    );
}
