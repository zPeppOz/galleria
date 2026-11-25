//! Writes a made-up rubrica into `$XDG_CONFIG_HOME/galleria`, with the import functions of the phase 1: five profiles
//! with the names of the mockup, three groups, two favourites and secrets that all start with `SEED-SECRET-`. For the
//! smoke test and the screen reader check (`lab/tests/gui-lib.sh`), never for the machine of a user.
//!
//! `seed-rubrica [--empty]` writes the store only, with no profile. It refuses to run without `XDG_CONFIG_HOME`, so
//! that it never touches `~/.config`.
//!
//! `seed-rubrica --fixtures <folder>` writes the files an import is tried on (`.ovpn` and `.conf`, with the files they
//! refer to, and the malicious profile of the laboratory) into that folder and nothing else; it does not touch the
//! rubrica and prints the paths, one per line, as `name=path`.

#[path = "../tests/support/fixtures.rs"]
mod fixtures;
#[path = "../tests/support/seed.rs"]
mod seed;

use std::{path::PathBuf, process::ExitCode};

fn write_fixtures(folder: &str) -> ExitCode {
    let files = fixtures::write_fixtures(&PathBuf::from(folder));
    for (name, path) in [
        ("inline", &files.inline),
        ("separate", &files.separate),
        ("malicious", &files.malicious),
        ("dns", &files.dns),
        ("outside", &files.outside),
        ("office", &files.office),
        ("server", &files.server),
    ] {
        println!("{name}={}", path.display());
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(at) = args.iter().position(|a| a == "--fixtures") {
        let Some(folder) = args.get(at + 1) else {
            eprintln!("seed-rubrica: --fixtures needs a folder");
            return ExitCode::FAILURE;
        };
        return write_fixtures(folder);
    }
    let empty = args.iter().any(|a| a == "--empty");
    let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) else {
        eprintln!("seed-rubrica: XDG_CONFIG_HOME is not set; it never writes to the real home");
        return ExitCode::FAILURE;
    };
    let dir = PathBuf::from(config).join("galleria");
    let seeded = seed::seed(&dir, empty);
    println!(
        "seed-rubrica: {} profiles in {}",
        seeded.profiles.len(),
        dir.display()
    );
    ExitCode::SUCCESS
}
