#![forbid(unsafe_code)]

fn main() -> anyhow::Result<std::process::ExitCode> {
    Ok(galleria_gui::run()?)
}
