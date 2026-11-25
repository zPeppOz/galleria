#![forbid(unsafe_code)]

fn main() -> anyhow::Result<()> {
    galleria_gui::run()?;
    Ok(())
}
