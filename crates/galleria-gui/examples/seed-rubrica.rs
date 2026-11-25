//! Writes a made-up rubrica into `$XDG_CONFIG_HOME/galleria`, with the import functions of the phase 1: five profiles
//! with the names of the mockup, three groups, two favourites and secrets that all start with `SEED-SECRET-`. For the
//! smoke test and the screen reader check (`lab/tests/gui-lib.sh`), never for the machine of a user.
//!
//! `seed-rubrica [--empty]` writes the store only, with no profile. It refuses to run without `XDG_CONFIG_HOME`, so
//! that it never touches `~/.config`.

#[path = "../tests/support/seed.rs"]
mod seed;

use std::{path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let empty = std::env::args().any(|a| a == "--empty");
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
