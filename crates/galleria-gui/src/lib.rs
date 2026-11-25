#![forbid(unsafe_code)]

//! Rust side of the Galleria GUI: builds the Tauri app that hosts the `ui/` frontend.

/// Builds the Tauri application from `tauri.conf.json` and runs it until the last window closes.
///
/// # Errors
///
/// Returns the Tauri error when the application cannot be built or crashes while running.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default().run(tauri::generate_context!())
}
