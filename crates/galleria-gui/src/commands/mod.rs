//! Tauri commands: thin wrappers over the modules that do not import Tauri. The list is declared in `build.rs`.

pub mod announce;
pub mod app;
pub mod daemon;
pub mod groups;
pub mod profiles;
pub mod ui_state;
